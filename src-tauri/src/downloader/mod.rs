pub mod m3u8;
mod task;

use crate::models::TextbookDownloadInfo;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::fs;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use task::{DownloadEventEmitter, DownloadStatus};

// 进行中的下载，以 URL 为键，用于取消/暂停（半成品保留，语义由前端决定）
static DOWNLOAD_TOKENS: Lazy<Mutex<HashMap<String, CancellationToken>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

async fn register_download_token(url: &str, token: CancellationToken) {
    DOWNLOAD_TOKENS.lock().await.insert(url.to_string(), token);
}

async fn cleanup_download_token(url: &str) {
    DOWNLOAD_TOKENS.lock().await.remove(url);
}

// 任务结束后的统一收尾：清理令牌；若因取消而结束，补发 cancelled 事件
// （在任务真正停下后才发，前端可安全地立即续传/清理半成品）
async fn finish_download(
    url: &str,
    app_handle: tauri::AppHandle,
    cancellation_token: &CancellationToken,
    result: &Result<String, String>,
) {
    cleanup_download_token(url).await;
    if result.is_err() && cancellation_token.is_cancelled() {
        DownloadEventEmitter::new(app_handle, url.to_string())
            .emit_status(DownloadStatus::Cancelled, 0);
    }
}

#[tauri::command]
pub async fn download_textbook(
    app_handle: tauri::AppHandle,
    textbook_info: TextbookDownloadInfo,
    token: Option<String>,
    download_path: String,
) -> Result<String, String> {
    let cancellation_token = CancellationToken::new();
    let url = textbook_info.url.clone();

    register_download_token(&url, cancellation_token.clone()).await;

    let result = task::run(
        app_handle.clone(),
        textbook_info,
        token,
        download_path,
        cancellation_token.clone(),
    )
    .await;

    finish_download(&url, app_handle, &cancellation_token, &result).await;
    result
}

#[tauri::command]
pub async fn download_course_resource(
    app_handle: tauri::AppHandle,
    resource: crate::models::CourseDownloadInfo,
    token: Option<String>,
    download_path: String,
    ffmpeg_path: Option<String>,
) -> Result<String, String> {
    let cancellation_token = CancellationToken::new();
    let url = resource.download_url.clone();

    register_download_token(&url, cancellation_token.clone()).await;

    let result = task::run_course(
        app_handle.clone(),
        resource,
        token,
        download_path,
        ffmpeg_path,
        cancellation_token.clone(),
    )
    .await;

    finish_download(&url, app_handle, &cancellation_token, &result).await;
    result
}

/// 停止进行中的下载。半成品（.part / .parts）一律保留：
/// 前端「暂停」直接复用本命令，「重新下载/删除」再调 remove_download_artifacts 清理。
#[tauri::command]
pub async fn cancel_download(url: String) -> Result<(), String> {
    if let Some(token) = DOWNLOAD_TOKENS.lock().await.remove(&url) {
        token.cancel();
        log::info!("已发送取消信号: {url}");
        Ok(())
    } else {
        Err(format!("没有进行中的下载: {url}"))
    }
}

/// 清理某任务的半成品（<final>.part 文件与 <final>.parts 切片目录），不动最终文件。
/// file_path 为任务开始时事件上报的目标路径。
#[tauri::command]
pub async fn remove_download_artifacts(file_path: String) -> Result<(), String> {
    if file_path.is_empty() {
        return Ok(());
    }
    let final_path = std::path::PathBuf::from(&file_path);

    let part = task::path_with_suffix(&final_path, ".part");
    if fs::try_exists(&part).await.unwrap_or(false) {
        fs::remove_file(&part)
            .await
            .map_err(|e| format!("删除半成品失败: {e}"))?;
    }

    let parts_dir = task::path_with_suffix(&final_path, ".parts");
    if fs::try_exists(&parts_dir).await.unwrap_or(false) {
        fs::remove_dir_all(&parts_dir)
            .await
            .map_err(|e| format!("删除切片缓存失败: {e}"))?;
    }

    Ok(())
}

/// 检测 ffmpeg 是否可用（设置页用）
#[tauri::command]
pub async fn check_ffmpeg(path: String) -> Result<bool, String> {
    Ok(m3u8::probe_ffmpeg(&path).await)
}
