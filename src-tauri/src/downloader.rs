use crate::models::TextbookDownloadInfo;
use bytes::Bytes;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use url::Url;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";
const PROGRESS_UPDATE_THRESHOLD: u64 = 1024 * 1024; // 1MB

pub static DOWNLOAD_TOKENS: Lazy<Arc<Mutex<HashMap<String, CancellationToken>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Debug, Clone)]
enum DownloadStatus {
    Downloading,
    Completed,
    Failed(String),
    Cancelled,
}

struct DownloadEventEmitter {
    app_handle: tauri::AppHandle,
    url: String,
}

impl DownloadEventEmitter {
    fn new(app_handle: tauri::AppHandle, url: String) -> Self {
        Self { app_handle, url }
    }

    fn emit_status(&self, status: DownloadStatus, progress: u32) {
        let event_data = match status {
            DownloadStatus::Downloading => json!({
                "url": self.url,
                "status": "downloading",
                "progress": progress,
            }),
            DownloadStatus::Completed => json!({
                "url": self.url,
                "status": "completed",
                "progress": 100,
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

    fn emit_progress(&self, progress: u32) {
        let _ = self.app_handle.emit(
            "download-progress",
            json!({
                "url": self.url,
                "progress": progress,
            }),
        );
    }

    fn emit_completed(&self, file_path: &str) {
        self.emit_status(DownloadStatus::Completed, 100);
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
        .and_then(|segments| segments.last())
        .and_then(|segment| segment.rsplit('.').next())
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default()
}

fn build_save_path(textbook_info: &TextbookDownloadInfo, download_path: &str) -> PathBuf {
    let mut base_save_path = PathBuf::from(download_path);

    if textbook_info.save_by_category {
        let labels = [
            &textbook_info.category_label,
            &textbook_info.subject_label,
            &textbook_info.version_label,
            &textbook_info.grade_label,
            &textbook_info.year_label,
        ];

        for label in labels.iter().filter_map(|l| l.as_ref()) {
            if !label.is_empty() {
                base_save_path.push(label);
            }
        }
    }

    base_save_path
}

fn create_request(
    client: &reqwest::Client,
    url: &Url,
    token: Option<&str>,
) -> reqwest::RequestBuilder {
    let mut request = client.get(url.clone());
    request = request.header(reqwest::header::USER_AGENT, USER_AGENT);

    if let Some(t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", t));
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
    file_path: &PathBuf,
    total_size: Option<u64>,
    cancellation_token: &CancellationToken,
    emitter: &DownloadEventEmitter,
) -> Result<(), String> {
    let mut downloaded_size: u64 = 0;
    let mut last_progress_update = 0u64;

    let mut file = fs::File::create(file_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    while let Some(chunk_result) = stream.next().await {
        if cancellation_token.is_cancelled() {
            let _ = fs::remove_file(file_path).await;
            return Err("Download cancelled".to_string());
        }

        let chunk = chunk_result.map_err(|e| {
            let progress = calculate_progress(downloaded_size, total_size);
            emitter.emit_status(
                DownloadStatus::Failed(format!("Error while downloading: {}", e)),
                progress,
            );
            format!("Error while downloading: {}", e)
        })?;

        let chunk_size = chunk.len() as u64;

        file.write_all(&chunk).await.map_err(|e| {
            let progress = calculate_progress(downloaded_size, total_size);
            emitter.emit_status(
                DownloadStatus::Failed(format!("Error while writing to file: {}", e)),
                progress,
            );
            format!("Error while writing to file: {}", e)
        })?;

        downloaded_size += chunk_size;

        if downloaded_size - last_progress_update >= PROGRESS_UPDATE_THRESHOLD
            || total_size.map_or(false, |total| downloaded_size >= total)
        {
            let progress = calculate_progress(downloaded_size, total_size);
            emitter.emit_progress(progress);
            last_progress_update = downloaded_size;
        }
    }

    Ok(())
}

async fn register_download_token(url: &str, token: CancellationToken) {
    let mut tokens = DOWNLOAD_TOKENS.lock().await;
    tokens.insert(url.to_string(), token);
}

async fn cleanup_download_token(url: &str) {
    let mut tokens = DOWNLOAD_TOKENS.lock().await;
    tokens.remove(url);
}

fn setup_cancellation_cleanup(
    url: String,
    cancellation_token: CancellationToken,
    app_handle: tauri::AppHandle,
) {
    tokio::spawn(async move {
        cancellation_token.cancelled().await;
        cleanup_download_token(&url).await;

        let emitter = DownloadEventEmitter::new(app_handle, url);
        emitter.emit_status(DownloadStatus::Cancelled, 0);
    });
}

pub async fn download_textbook_internal(
    app_handle: tauri::AppHandle,
    textbook_info: TextbookDownloadInfo,
    token: Option<String>,
    download_path: String,
    cancellation_token: CancellationToken,
) -> Result<String, String> {
    let url = &textbook_info.url;
    let title = &textbook_info.title;

    println!("Attempting to download '{}' from: {}", title, url);
    println!("Using base download path: {}", download_path);

    if cancellation_token.is_cancelled() {
        println!("Download cancelled before starting: {}", url);
        return Err("Download cancelled".to_string());
    }

    if download_path.is_empty() {
        println!("Download path not provided for {}", url);
        return Err("Download path is not provided.".to_string());
    }

    let parsed_url = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

    let file_extension = extract_file_extension(&parsed_url);
    let base_save_path = build_save_path(&textbook_info, &download_path);

    if !base_save_path.exists() {
        fs::create_dir_all(&base_save_path)
            .await
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
    }

    let filename = format!("{}{}", title, file_extension);
    let save_path = base_save_path.join(&filename);

    let emitter = DownloadEventEmitter::new(app_handle, url.clone());
    emitter.emit_status(DownloadStatus::Downloading, 0);

    let client = reqwest::Client::new();
    let request = create_request(&client, &parsed_url, token.as_deref());

    let response = request.send().await.map_err(|e| {
        let error_msg = format!("Download failed: {}", e);
        emitter.emit_status(DownloadStatus::Failed(error_msg.clone()), 0);
        error_msg
    })?;

    if !response.status().is_success() {
        let error_msg = format!("Download failed with status: {}", response.status());
        emitter.emit_status(DownloadStatus::Failed(error_msg.clone()), 0);
        return Err(error_msg);
    }

    let total_size = response.content_length();
    let stream = response.bytes_stream();

    process_download_stream(
        stream,
        &save_path,
        total_size,
        &cancellation_token,
        &emitter,
    )
    .await?;

    println!("Download finished successfully: {}", save_path.display());

    let file_path_str = save_path.to_string_lossy().into_owned();
    emitter.emit_completed(&file_path_str);

    Ok(file_path_str)
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

    let result = download_textbook_internal(
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
    println!("Attempting to cancel download: {}", url);
    let mut tokens = DOWNLOAD_TOKENS.lock().await;
    if let Some(token) = tokens.remove(&url) {
        token.cancel();
        println!("Cancellation signal sent for: {}", url);
        Ok(())
    } else {
        eprintln!("No active download found for: {}", url);
        Err(format!("No active download found for: {}", url))
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
    println!(
        "Attempting to batch download {} files with {} threads to: {}",
        textbooks_to_download.len(),
        thread_count,
        download_path
    );

    if textbooks_to_download.is_empty() {
        return Err("No textbooks provided for download".to_string());
    }

    if download_path.is_empty() {
        return Err("Download path is not provided.".to_string());
    }

    if thread_count == 0 {
        return Err("Thread count must be greater than 0".to_string());
    }

    let semaphore = Arc::new(tokio::sync::Semaphore::new(thread_count));
    let mut join_handles = Vec::with_capacity(textbooks_to_download.len());

    for textbook in textbooks_to_download {
        let app_handle_clone = app_handle.clone();
        let token_clone = token.clone();
        let download_path_clone = download_path.clone();
        let url = textbook.url.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        let cancellation_token = CancellationToken::new();

        register_download_token(&url, cancellation_token.clone()).await;

        setup_cancellation_cleanup(
            url.clone(),
            cancellation_token.clone(),
            app_handle_clone.clone(),
        );

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone
                .acquire()
                .await
                .expect("Failed to acquire semaphore permit");

            let result = if cancellation_token.is_cancelled() {
                println!("Download cancelled before starting: {}", url);
                Err("Download cancelled".to_string())
            } else {
                download_textbook_internal(
                    app_handle_clone,
                    textbook,
                    token_clone,
                    download_path_clone,
                    cancellation_token,
                )
                .await
            };

            cleanup_download_token(&url).await;
            result
        });

        join_handles.push(handle);
    }

    let mut all_succeeded = true;
    for handle in join_handles {
        match handle.await {
            Ok(download_result) => {
                if download_result.is_err() {
                    all_succeeded = false;
                }
            }
            Err(join_err) => {
                eprintln!("Task join error: {}", join_err);
                all_succeeded = false;
            }
        }
    }

    if all_succeeded {
        println!("All batch downloads finished successfully.");
        let _ = app_handle.emit(
            "batch-download-completed",
            json!({
                "downloadPath": download_path
            }),
        );
    } else {
        eprintln!("Some batch downloads failed.");
        let _ = app_handle.emit(
            "batch-download-failed",
            json!({
                "downloadPath": download_path
            }),
        );
    }

    Ok(())
}
