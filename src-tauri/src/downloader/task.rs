use crate::api::books;
use crate::http::CLIENT;
use crate::models::TextbookDownloadInfo;
use bytes::Bytes;
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

    pub(super) fn emit_progress(&self, progress: u32) {
        let _ = self.app_handle.emit(
            "download-progress",
            json!({
                "url": self.url,
                "progress": progress,
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

async fn process_download_stream(
    mut stream: impl futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    file_path: &Path,
    total_size: Option<u64>,
    cancellation_token: &CancellationToken,
    emitter: &DownloadEventEmitter,
) -> Result<(), String> {
    let mut downloaded_size: u64 = 0;
    let mut last_progress_update = 0u64;

    let file = fs::File::create(file_path)
        .await
        .map_err(|e| format!("创建文件失败: {e}"))?;
    let mut writer = BufWriter::with_capacity(WRITE_BUFFER_SIZE, file);

    while let Some(chunk_result) = stream.next().await {
        if cancellation_token.is_cancelled() {
            drop(writer);
            let _ = fs::remove_file(file_path).await;
            return Err("下载已取消".to_string());
        }

        let chunk = chunk_result.map_err(|e| {
            let progress = calculate_progress(downloaded_size, total_size);
            let msg = format!("下载出错: {e}");
            emitter.emit_status(DownloadStatus::Failed(msg.clone()), progress);
            msg
        })?;

        writer.write_all(&chunk).await.map_err(|e| {
            let progress = calculate_progress(downloaded_size, total_size);
            let msg = format!("写入文件失败: {e}");
            emitter.emit_status(DownloadStatus::Failed(msg.clone()), progress);
            msg
        })?;

        downloaded_size += chunk.len() as u64;

        if downloaded_size - last_progress_update >= PROGRESS_UPDATE_THRESHOLD
            || total_size.is_some_and(|total| downloaded_size >= total)
        {
            emitter.emit_progress(calculate_progress(downloaded_size, total_size));
            last_progress_update = downloaded_size;
        }
    }

    writer
        .flush()
        .await
        .map_err(|e| format!("写入文件失败: {e}"))?;

    Ok(())
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

async fn send_request(url: &Url, token: Option<&str>) -> Result<reqwest::Response, String> {
    let response = create_request(url, token)
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                "下载失败：需要有效的 Access Token（请在「设置」中填写，或令牌可能已过期）"
                    .to_string()
            } else {
                format!("下载失败: HTTP {status}")
            },
        );
    }
    Ok(response)
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

    // 依次尝试候选地址，取第一个成功的响应
    let mut resolved: Option<(Url, reqwest::Response)> = None;
    let mut last_error = "下载失败".to_string();
    for candidate in download_candidates(url).await {
        if cancellation_token.is_cancelled() {
            return Err("下载已取消".to_string());
        }
        match Url::parse(&candidate) {
            Ok(parsed) => match send_request(&parsed, token.as_deref()).await {
                Ok(response) => {
                    resolved = Some((parsed, response));
                    break;
                }
                Err(e) => {
                    log::warn!("地址不可用 {candidate}: {e}");
                    last_error = e;
                }
            },
            Err(e) => last_error = format!("无效的 URL: {e}"),
        }
    }

    let Some((final_url, response)) = resolved else {
        emitter.emit_status(DownloadStatus::Failed(last_error.clone()), 0);
        return Err(last_error);
    };

    let base_save_path = build_save_path(&textbook_info, &download_path);
    if !base_save_path.exists() {
        fs::create_dir_all(&base_save_path)
            .await
            .map_err(|e| format!("创建下载目录失败: {e}"))?;
    }

    let filename = format!(
        "{}{}",
        textbook_info.title,
        extract_file_extension(&final_url)
    );
    let save_path = base_save_path.join(&filename);

    let total_size = response.content_length();
    process_download_stream(
        response.bytes_stream(),
        &save_path,
        total_size,
        &cancellation_token,
        &emitter,
    )
    .await?;

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

/// 下载单个课程资源：视频走 m3u8 解密流程，其余走普通流式下载。
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

    // 同一课程的多个资源归到以课程标题命名的子目录
    let mut base_save_path = PathBuf::from(&download_path);
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

    let final_path = if resource.is_video {
        // 视频合成后的真实路径可能是 .mp4（ffmpeg 转封装成功）或 .ts（回退）
        super::m3u8::download(
            &url,
            token.as_deref(),
            &save_path,
            ffmpeg_path.as_deref(),
            &cancellation_token,
            &emitter,
        )
        .await?
    } else {
        let parsed = Url::parse(&url).map_err(|e| format!("无效的 URL: {e}"))?;
        let response = send_request(&parsed, token.as_deref()).await.inspect_err(|e| {
            emitter.emit_status(DownloadStatus::Failed(e.clone()), 0);
        })?;
        let total_size = response.content_length();
        process_download_stream(
            response.bytes_stream(),
            &save_path,
            total_size,
            &cancellation_token,
            &emitter,
        )
        .await?;
        save_path
    };

    log::info!("课程资源下载完成: {}", final_path.display());
    let file_path_str = final_path.to_string_lossy().into_owned();
    emitter.emit_completed(&file_path_str);
    Ok(file_path_str)
}
