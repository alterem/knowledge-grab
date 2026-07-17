use crate::api::books;
use crate::http::CLIENT;
use crate::models::TextbookDownloadInfo;
use futures_util::StreamExt;
use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tokio::fs;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;
use url::Url;

pub(super) const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";
// 进度事件按下载量节流，写文件用缓冲减少 syscall
const PROGRESS_UPDATE_THRESHOLD: u64 = 1024 * 1024;
const WRITE_BUFFER_SIZE: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub(super) enum DownloadStatus {
    Downloading,
    Failed(String),
    Cancelled,
}

pub(super) struct DownloadEventEmitter {
    app_handle: tauri::AppHandle,
    url: String,
}

impl DownloadEventEmitter {
    pub(super) fn new(app_handle: tauri::AppHandle, url: String) -> Self {
        Self { app_handle, url }
    }

    pub(super) fn emit_status(&self, status: DownloadStatus, progress: u32) {
        let event_data = match status {
            DownloadStatus::Downloading => json!({
                "url": self.url,
                "status": "downloading",
                "progress": progress,
            }),
            DownloadStatus::Failed(error) => json!({
                "url": self.url,
                "status": "failed",
                "error": error,
                "progress": progress,
            }),
            DownloadStatus::Cancelled => json!({
                "url": self.url,
                "status": "cancelled",
                "progress": 0,
            }),
        };

        let _ = self.app_handle.emit("download-status", event_data);
    }

    // 进度附带字节数，前端据此计算速度；总大小未知时（m3u8 按切片计数）为 null
    pub(super) fn emit_progress(&self, progress: u32, downloaded_bytes: u64, total_bytes: Option<u64>) {
        let _ = self.app_handle.emit(
            "download-progress",
            json!({
                "url": self.url,
                "progress": progress,
                "downloadedBytes": downloaded_bytes,
                "totalBytes": total_bytes,
            }),
        );
    }

    // 任务开始时先把目标路径告知前端：中断后续传、清理半成品都以它为锚点
    pub(super) fn emit_target_path(&self, file_path: &Path) {
        let _ = self.app_handle.emit(
            "download-status",
            json!({
                "url": self.url,
                "status": "downloading",
                "progress": 0,
                "filePath": file_path.to_string_lossy(),
            }),
        );
    }

    pub(super) fn emit_completed(&self, file_path: &str) {
        let _ = self.app_handle.emit(
            "download-status",
            json!({
                "url": self.url,
                "status": "completed",
                "progress": 100,
                "filePath": file_path,
            }),
        );
    }
}

// 在文件名末尾追加后缀（"a/b.mp4" + ".parts" → "a/b.mp4.parts"），半成品统一命名
pub(super) fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

fn extract_file_extension(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .and_then(|segment| segment.rsplit('.').next())
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default()
}

// 「按分类保存」时用各级标签名拼出子目录
fn build_save_path(info: &TextbookDownloadInfo, download_path: &str) -> PathBuf {
    let mut path = PathBuf::from(download_path);

    if info.save_by_category {
        let labels = [
            &info.category_label,
            &info.subject_label,
            &info.version_label,
            &info.grade_label,
            &info.year_label,
        ];
        for label in labels.iter().filter_map(|l| l.as_ref()) {
            if !label.is_empty() {
                path.push(label);
            }
        }
    }

    path
}

fn create_request(url: &Url, token: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = CLIENT
        .get(url.clone())
        .header(reqwest::header::USER_AGENT, USER_AGENT);

    if let Some(t) = token {
        // 私有桶用 ND UC 的 MAC 鉴权头，服务端只校验 token id，
        // nonce/mac 可为占位值（Bearer 会被 400 拒绝）
        request = request.header("x-nd-auth", format!("MAC id=\"{t}\",nonce=\"0\",mac=\"0\""));
    }

    request
}

fn calculate_progress(downloaded: u64, total: Option<u64>) -> u32 {
    match total {
        Some(total) if total > 0 => (downloaded as f64 / total as f64 * 100.0) as u32,
        _ => 0,
    }
}

fn map_http_error(status: reqwest::StatusCode) -> String {
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        "下载失败：需要有效的 Access Token（请在「设置」中填写，或令牌可能已过期）".to_string()
    } else {
        format!("下载失败: HTTP {status}")
    }
}

struct ResumableResponse {
    response: reqwest::Response,
    // 实际续传起点：服务器返回 206 时为 .part 现有长度，否则 0（从头写）
    resume_from: u64,
    // 最终文件总大小（续传时 = 剩余长度 + 起点）
    total_size: Option<u64>,
}

// 发起下载请求；part_path 已有半成品时带 Range 续传。
// 服务器忽略 Range（200）则从头下载；416（起点越界，远端文件已变或早已下完）
// 时丢弃半成品重发一次完整请求。
async fn open_resumable(
    url: &Url,
    token: Option<&str>,
    part_path: &Path,
) -> Result<ResumableResponse, String> {
    let mut resume_from = fs::metadata(part_path).await.map(|m| m.len()).unwrap_or(0);

    loop {
        let mut request = create_request(url, token);
        if resume_from > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }
        let response = request.send().await.map_err(|e| format!("下载失败: {e}"))?;
        let status = response.status();

        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && resume_from > 0 {
            log::warn!("Range 起点越界，丢弃半成品重新下载: {}", part_path.display());
            let _ = fs::remove_file(part_path).await;
            resume_from = 0;
            continue;
        }
        if !status.is_success() {
            return Err(map_http_error(status));
        }

        if status != reqwest::StatusCode::PARTIAL_CONTENT {
            resume_from = 0;
        } else {
            log::info!("从 {resume_from} 字节处续传: {url}");
        }
        let total_size = response.content_length().map(|len| len + resume_from);
        return Ok(ResumableResponse {
            response,
            resume_from,
            total_size,
        });
    }
}

// 把响应流写入 .part 文件（续传时追加）。取消/出错都保留 .part 供下次续传；
// 状态事件（failed/cancelled）由调用方统一发射，这里只发进度。
async fn stream_to_part_file(
    opened: ResumableResponse,
    part_path: &Path,
    cancellation_token: &CancellationToken,
    emitter: &DownloadEventEmitter,
) -> Result<(), String> {
    let ResumableResponse {
        response,
        resume_from,
        total_size,
    } = opened;
    let mut stream = response.bytes_stream();

    let file = if resume_from > 0 {
        fs::OpenOptions::new().append(true).open(part_path).await
    } else {
        fs::File::create(part_path).await
    }
    .map_err(|e| format!("创建文件失败: {e}"))?;
    let mut writer = BufWriter::with_capacity(WRITE_BUFFER_SIZE, file);

    let mut downloaded_size = resume_from;
    let mut last_progress_update = resume_from;
    if resume_from > 0 {
        // 让进度条直接跳到续传起点
        emitter.emit_progress(
            calculate_progress(downloaded_size, total_size),
            downloaded_size,
            total_size,
        );
    }

    while let Some(chunk_result) = stream.next().await {
        if cancellation_token.is_cancelled() {
            let _ = writer.flush().await;
            return Err("下载已取消".to_string());
        }

        let chunk = chunk_result.map_err(|e| format!("下载出错: {e}"))?;
        writer
            .write_all(&chunk)
            .await
            .map_err(|e| format!("写入文件失败: {e}"))?;

        downloaded_size += chunk.len() as u64;

        if downloaded_size - last_progress_update >= PROGRESS_UPDATE_THRESHOLD
            || total_size.is_some_and(|total| downloaded_size >= total)
        {
            emitter.emit_progress(
                calculate_progress(downloaded_size, total_size),
                downloaded_size,
                total_size,
            );
            last_progress_update = downloaded_size;
        }
    }

    writer
        .flush()
        .await
        .map_err(|e| format!("写入文件失败: {e}"))?;

    Ok(())
}

// 完整的可续传下载：<final>.part → 下载/续传 → 改名为最终文件
async fn download_resumable(
    url: &Url,
    token: Option<&str>,
    final_path: &Path,
    cancellation_token: &CancellationToken,
    emitter: &DownloadEventEmitter,
) -> Result<(), String> {
    let part_path = path_with_suffix(final_path, ".part");
    let opened = open_resumable(url, token, &part_path).await?;
    stream_to_part_file(opened, &part_path, cancellation_token, emitter).await?;
    // Windows 上 rename 不能覆盖已存在文件，重新下载场景先移除旧文件
    let _ = fs::remove_file(final_path).await;
    fs::rename(&part_path, final_path)
        .await
        .map_err(|e| format!("保存文件失败: {e}"))
}

// 候选下载地址：详情声明的源 PDF 优先（部分教材的 pkg 没有 pdf.pdf 别名），
// 传入的构造 URL 兜底
async fn download_candidates(url: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(id) = books::resource_id_from_url(url) {
        candidates = books::resolve_source_pdf_urls(id).await;
    }
    if !candidates.iter().any(|c| c == url) {
        candidates.push(url.to_string());
    }
    candidates
}

pub(super) async fn run(
    app_handle: tauri::AppHandle,
    textbook_info: TextbookDownloadInfo,
    token: Option<String>,
    download_path: String,
    cancellation_token: CancellationToken,
) -> Result<String, String> {
    let url = &textbook_info.url;
    log::info!("开始下载《{}》: {url}", textbook_info.title);

    if cancellation_token.is_cancelled() {
        return Err("下载已取消".to_string());
    }
    if download_path.is_empty() {
        return Err("未设置下载路径".to_string());
    }

    let emitter = DownloadEventEmitter::new(app_handle, url.clone());
    emitter.emit_status(DownloadStatus::Downloading, 0);

    let base_save_path = build_save_path(&textbook_info, &download_path);
    if !base_save_path.exists() {
        fs::create_dir_all(&base_save_path)
            .await
            .map_err(|e| format!("创建下载目录失败: {e}"))?;
    }

    // 依次尝试候选地址：初始请求失败换下一个；请求成功后按其扩展名确定目标文件，
    // 已有 .part 半成品时自动续传
    let mut last_error = "下载失败".to_string();
    let mut completed: Option<PathBuf> = None;
    for candidate in download_candidates(url).await {
        if cancellation_token.is_cancelled() {
            return Err("下载已取消".to_string());
        }
        let parsed = match Url::parse(&candidate) {
            Ok(parsed) => parsed,
            Err(e) => {
                last_error = format!("无效的 URL: {e}");
                continue;
            }
        };

        let filename = format!(
            "{}{}",
            textbook_info.title,
            extract_file_extension(&parsed)
        );
        let save_path = base_save_path.join(&filename);
        let part_path = path_with_suffix(&save_path, ".part");

        let opened = match open_resumable(&parsed, token.as_deref(), &part_path).await {
            Ok(opened) => opened,
            Err(e) => {
                log::warn!("地址不可用 {candidate}: {e}");
                last_error = e;
                continue;
            }
        };

        emitter.emit_target_path(&save_path);

        let result = async {
            stream_to_part_file(opened, &part_path, &cancellation_token, &emitter).await?;
            // Windows 上 rename 不能覆盖已存在文件，重新下载场景先移除旧文件
            let _ = fs::remove_file(&save_path).await;
            fs::rename(&part_path, &save_path)
                .await
                .map_err(|e| format!("保存文件失败: {e}"))
        }
        .await;

        if let Err(e) = result {
            if !cancellation_token.is_cancelled() {
                emitter.emit_status(DownloadStatus::Failed(e.clone()), 0);
            }
            return Err(e);
        }

        completed = Some(save_path);
        break;
    }

    let Some(save_path) = completed else {
        emitter.emit_status(DownloadStatus::Failed(last_error.clone()), 0);
        return Err(last_error);
    };

    log::info!("下载完成: {}", save_path.display());

    let file_path_str = save_path.to_string_lossy().into_owned();
    emitter.emit_completed(&file_path_str);

    Ok(file_path_str)
}

// 文件名/目录名清洗：去掉路径非法字符，避免拼接出非法路径
fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "未命名".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 下载单个课程资源：视频走 m3u8 解密流程（按切片续传），其余走普通流式下载（Range 续传）。
/// 事件以 resource.download_url 为键，与前端下载状态仓库对应。
pub(super) async fn run_course(
    app_handle: tauri::AppHandle,
    resource: crate::models::CourseDownloadInfo,
    token: Option<String>,
    download_path: String,
    ffmpeg_path: Option<String>,
    cancellation_token: CancellationToken,
) -> Result<String, String> {
    let url = resource.download_url.clone();
    log::info!("开始下载课程资源《{}》: {url}", resource.title);

    if cancellation_token.is_cancelled() {
        return Err("下载已取消".to_string());
    }
    if download_path.is_empty() {
        return Err("未设置下载路径".to_string());
    }

    let emitter = DownloadEventEmitter::new(app_handle, url.clone());
    emitter.emit_status(DownloadStatus::Downloading, 0);

    // 「按分类保存」时先按分类目录段分层，同一课程的多个资源再归到课程标题子目录
    let mut base_save_path = PathBuf::from(&download_path);
    if resource.save_by_category {
        for seg in resource.category_path.iter().filter(|s| !s.trim().is_empty()) {
            base_save_path.push(sanitize_name(seg));
        }
    }
    if let Some(course) = resource.course_title.as_deref().filter(|s| !s.is_empty()) {
        base_save_path.push(sanitize_name(course));
    }
    if !base_save_path.exists() {
        fs::create_dir_all(&base_save_path)
            .await
            .map_err(|e| format!("创建下载目录失败: {e}"))?;
    }

    let title = sanitize_name(&resource.title);
    let ext = if resource.format.is_empty() {
        "bin".to_string()
    } else {
        resource.format.clone()
    };
    let save_path = base_save_path.join(format!("{title}.{ext}"));
    emitter.emit_target_path(&save_path);

    let download_result: Result<PathBuf, String> = if resource.is_video {
        // 视频合成后的真实路径可能是 .mp4（ffmpeg 转封装成功）或 .ts（回退）
        super::m3u8::download(
            &url,
            token.as_deref(),
            &save_path,
            ffmpeg_path.as_deref(),
            &cancellation_token,
            &emitter,
        )
        .await
    } else {
        let parsed = Url::parse(&url).map_err(|e| format!("无效的 URL: {e}"))?;
        download_resumable(
            &parsed,
            token.as_deref(),
            &save_path,
            &cancellation_token,
            &emitter,
        )
        .await
        .map(|_| save_path)
    };

    let final_path = download_result.inspect_err(|e| {
        // 取消不算失败，cancelled 事件由命令包装层统一补发
        if !cancellation_token.is_cancelled() {
            emitter.emit_status(DownloadStatus::Failed(e.clone()), 0);
        }
    })?;

    log::info!("课程资源下载完成: {}", final_path.display());
    let file_path_str = final_path.to_string_lossy().into_owned();
    emitter.emit_completed(&file_path_str);
    Ok(file_path_str)
}
