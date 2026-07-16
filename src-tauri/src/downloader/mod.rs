pub mod m3u8;
mod task;

use crate::models::TextbookDownloadInfo;
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

use task::{DownloadEventEmitter, DownloadStatus};

// 进行中的下载，以 URL 为键，用于取消
static DOWNLOAD_TOKENS: Lazy<Mutex<HashMap<String, CancellationToken>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

async fn register_download_token(url: &str, token: CancellationToken) {
    DOWNLOAD_TOKENS.lock().await.insert(url.to_string(), token);
}

async fn cleanup_download_token(url: &str) {
    DOWNLOAD_TOKENS.lock().await.remove(url);
}

fn setup_cancellation_cleanup(
    url: String,
    cancellation_token: CancellationToken,
    app_handle: tauri::AppHandle,
) {
    tokio::spawn(async move {
        cancellation_token.cancelled().await;
        cleanup_download_token(&url).await;
        DownloadEventEmitter::new(app_handle, url).emit_status(DownloadStatus::Cancelled, 0);
    });
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
    setup_cancellation_cleanup(url.clone(), cancellation_token.clone(), app_handle.clone());

    let result = task::run(
        app_handle,
        textbook_info,
        token,
        download_path,
        cancellation_token,
    )
    .await;

    cleanup_download_token(&url).await;
    result
}

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

#[tauri::command]
pub async fn batch_download_textbooks(
    app_handle: tauri::AppHandle,
    textbooks_to_download: Vec<TextbookDownloadInfo>,
    token: Option<String>,
    download_path: String,
    thread_count: usize,
) -> Result<(), String> {
    log::info!(
        "批量下载 {} 个文件，并发 {}，目录: {download_path}",
        textbooks_to_download.len(),
        thread_count
    );

    if textbooks_to_download.is_empty() {
        return Err("没有可下载的教材".to_string());
    }
    if download_path.is_empty() {
        return Err("未设置下载路径".to_string());
    }
    if thread_count == 0 {
        return Err("并发数必须大于 0".to_string());
    }

    let semaphore = Arc::new(Semaphore::new(thread_count));
    let mut join_handles = Vec::with_capacity(textbooks_to_download.len());

    for textbook in textbooks_to_download {
        let app_handle = app_handle.clone();
        let token = token.clone();
        let download_path = download_path.clone();
        let url = textbook.url.clone();
        let semaphore = Arc::clone(&semaphore);
        let cancellation_token = CancellationToken::new();

        register_download_token(&url, cancellation_token.clone()).await;
        setup_cancellation_cleanup(url.clone(), cancellation_token.clone(), app_handle.clone());

        join_handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.expect("信号量已关闭");

            let result = if cancellation_token.is_cancelled() {
                Err("下载已取消".to_string())
            } else {
                task::run(
                    app_handle,
                    textbook,
                    token,
                    download_path,
                    cancellation_token,
                )
                .await
            };

            cleanup_download_token(&url).await;
            result
        }));
    }

    let mut all_succeeded = true;
    for handle in join_handles {
        match handle.await {
            Ok(result) => all_succeeded &= result.is_ok(),
            Err(e) => {
                log::error!("下载任务异常: {e}");
                all_succeeded = false;
            }
        }
    }

    let event = if all_succeeded {
        "batch-download-completed"
    } else {
        "batch-download-failed"
    };
    let _ = app_handle.emit(event, json!({ "downloadPath": download_path }));

    Ok(())
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
    setup_cancellation_cleanup(url.clone(), cancellation_token.clone(), app_handle.clone());

    let result = task::run_course(
        app_handle,
        resource,
        token,
        download_path,
        ffmpeg_path,
        cancellation_token,
    )
    .await;

    cleanup_download_token(&url).await;
    result
}

#[tauri::command]
pub async fn batch_download_course_resources(
    app_handle: tauri::AppHandle,
    resources: Vec<crate::models::CourseDownloadInfo>,
    token: Option<String>,
    download_path: String,
    ffmpeg_path: Option<String>,
    thread_count: usize,
) -> Result<(), String> {
    if resources.is_empty() {
        return Err("没有可下载的资源".to_string());
    }
    if download_path.is_empty() {
        return Err("未设置下载路径".to_string());
    }
    let thread_count = thread_count.max(1);

    let semaphore = Arc::new(Semaphore::new(thread_count));
    let mut join_handles = Vec::with_capacity(resources.len());

    for resource in resources {
        let app_handle = app_handle.clone();
        let token = token.clone();
        let download_path = download_path.clone();
        let ffmpeg_path = ffmpeg_path.clone();
        let url = resource.download_url.clone();
        let semaphore = Arc::clone(&semaphore);
        let cancellation_token = CancellationToken::new();

        register_download_token(&url, cancellation_token.clone()).await;
        setup_cancellation_cleanup(url.clone(), cancellation_token.clone(), app_handle.clone());

        join_handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.expect("信号量已关闭");

            let result = if cancellation_token.is_cancelled() {
                Err("下载已取消".to_string())
            } else {
                task::run_course(
                    app_handle,
                    resource,
                    token,
                    download_path,
                    ffmpeg_path,
                    cancellation_token,
                )
                .await
            };

            cleanup_download_token(&url).await;
            result
        }));
    }

    let mut all_succeeded = true;
    for handle in join_handles {
        match handle.await {
            Ok(result) => all_succeeded &= result.is_ok(),
            Err(e) => {
                log::error!("下载任务异常: {e}");
                all_succeeded = false;
            }
        }
    }

    let event = if all_succeeded {
        "batch-download-completed"
    } else {
        "batch-download-failed"
    };
    let _ = app_handle.emit(event, json!({ "downloadPath": download_path }));

    Ok(())
}

/// 检测 ffmpeg 是否可用（设置页用）
#[tauri::command]
pub async fn check_ffmpeg(path: String) -> Result<bool, String> {
    Ok(m3u8::probe_ffmpeg(&path).await)
}
