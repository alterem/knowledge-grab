use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use once_cell::sync::Lazy;
use tauri::Emitter;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use reqwest;
use tokio::fs;
use url;
use serde_json::json;
use crate::models::TextbookDownloadInfo;

pub static DOWNLOAD_TOKENS: Lazy<Arc<Mutex<HashMap<String, CancellationToken>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

pub async fn download_textbook_internal(app_handle: tauri::AppHandle, textbook_info: TextbookDownloadInfo, token: Option<String>, download_path: String, cancellation_token: CancellationToken) -> Result<String, String> {
    println!("Attempting to download '{}' from: {}", textbook_info.title, textbook_info.url);
    println!("Using base download path: {}", download_path);

    let url = textbook_info.url.clone();
    let title = textbook_info.title.clone();

    if cancellation_token.is_cancelled() {
        println!("Download cancelled before starting: {}", url);
        return Err("Download cancelled".to_string());
    }

    if download_path.is_empty() {
        println!("Download path not provided for {}", url);
        return Err("Download path is not provided.".to_string());
    }

    let parsed_url = url::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;

    let file_extension = parsed_url
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|segment| segment.rsplit('.').next())
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_else(|| "".to_string());

    let mut base_save_path = std::path::PathBuf::from(&download_path);

    if textbook_info.save_by_category {
        if let Some(category) = textbook_info.category_label {
            if !category.is_empty() {
                base_save_path.push(category);
            }
        }
        if let Some(subject) = textbook_info.subject_label {
            if !subject.is_empty() {
                base_save_path.push(subject);
            }
        }
        if let Some(version) = textbook_info.version_label {
            if !version.is_empty() {
                base_save_path.push(version);
            }
        }
        if let Some(grade) = textbook_info.grade_label {
            if !grade.is_empty() {
                base_save_path.push(grade);
            }
        }
        if let Some(year) = textbook_info.year_label {
            if !year.is_empty() {
                base_save_path.push(year);
            }
        }
    }

    if !base_save_path.exists() {
        fs::create_dir_all(&base_save_path)
            .await
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
    }

    let filename = format!("{}{}", title, file_extension);
    let save_path = base_save_path.join(&filename);
    let client = reqwest::Client::new();
    let mut request = client.get(parsed_url.clone());
    request = request.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");

    if let Some(t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", t));
    }

    app_handle.emit("download-status", json!({
        "url": url.clone(),
        "status": "downloading",
        "progress": 0,
    })).unwrap();

    let response = request.send()
        .await
        .map_err(|e| {
            app_handle.emit("download-status", json!({
                "url": url.clone(),
                "status": "failed",
                "error": format!("Download failed: {}", e),
                "progress": 0,
            })).unwrap();
            format!("Download failed: {}", e)
        })?;
    if !response.status().is_success() {
        let error_msg = format!("Download failed with status: {}", response.status());
        app_handle.emit("download-status", json!({
            "url": url.clone(),
            "status": "failed",
            "error": error_msg.clone(),
            "progress": 0,
        })).unwrap();
        return Err(error_msg);
    }

    let total_size = response.content_length();
    let mut downloaded_size: u64 = 0;
    let mut file = fs::File::create(&save_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancellation_token.is_cancelled() {
            println!("Download cancelled during stream processing: {}", url);
            let _ = fs::remove_file(&save_path).await;
            return Err("Download cancelled".to_string());
        }

        let chunk = chunk.map_err(|e| {
            let _ = app_handle.emit("download-status", json!({
                "url": url.clone(),
                "status": "failed",
                "error": format!("Error while downloading: {}", e),
                "progress": (downloaded_size as f64 / total_size.unwrap_or(1) as f64 * 100.0) as u32,
            }));
            format!("Error while downloading: {}", e)
        })?;
        let chunk_size = chunk.len() as u64;
        if cancellation_token.is_cancelled() {
            println!("Download cancelled during file writing: {}", url);
            let _ = fs::remove_file(&save_path).await;
            return Err("Download cancelled".to_string());
        }

        file.write_all(&chunk)
            .await
            .map_err(|e| {
                let _ = app_handle.emit("download-status", json!({
                    "url": url.clone(),
                    "status": "failed",
                    "error": format!("Error while writing to file: {}", e),
                    "progress": (downloaded_size as f64 / total_size.unwrap_or(1) as f64 * 100.0) as u32,
                }));
                format!("Error while writing to file: {}", e)
            })?;
        downloaded_size += chunk_size;

        if let Some(total) = total_size {
            let progress = (downloaded_size as f64 / total as f64 * 100.0) as u32;
            let _ = app_handle.emit("download-progress", json!({
                "url": url.clone(),
                "progress": progress,
            }));
        }
    }

    println!("Download finished successfully: {}", save_path.display());

    let _ = app_handle.emit("download-status", json!({
        "url": url.clone(),
        "status": "completed",
        "progress": 100,
        "filePath": save_path.to_string_lossy().into_owned(),
    }));

    Ok(save_path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn download_textbook(app_handle: tauri::AppHandle, textbook_info: TextbookDownloadInfo, token: Option<String>, download_path: String) -> Result<String, String> {
    let cancellation_token = CancellationToken::new();
    {
        let mut tokens = DOWNLOAD_TOKENS.lock().await;
        tokens.insert(textbook_info.url.clone(), cancellation_token.clone());
    }

    let url_clone_for_cleanup = textbook_info.url.clone();
    let cleanup_token = cancellation_token.clone();
    let app_handle_clone_for_cleanup = app_handle.clone();
    tokio::spawn(async move {
        cleanup_token.cancelled().await;
        let mut tokens = DOWNLOAD_TOKENS.lock().await;
        tokens.remove(&url_clone_for_cleanup);
        let _ = app_handle_clone_for_cleanup.emit("download-status", json!({
            "url": url_clone_for_cleanup,
            "status": "cancelled",
            "progress": 0,
        }));
    });

    download_textbook_internal(app_handle, textbook_info, token, download_path, cancellation_token).await
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
pub async fn batch_download_textbooks(app_handle: tauri::AppHandle, textbooks_to_download: Vec<TextbookDownloadInfo>, token: Option<String>, download_path: String, thread_count: usize) -> Result<(), String> {
    println!("Attempting to batch download {} files with {} threads to: {}", textbooks_to_download.len(), thread_count, download_path);

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

    let mut join_handles = vec![];

    for textbook in textbooks_to_download {
        let app_handle_clone = app_handle.clone();
        let token_clone = token.clone();
        let download_path_clone = download_path.clone();
        let textbook_info_clone = textbook.clone();
        let url_clone = textbook_info_clone.url.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        let cancellation_token = CancellationToken::new();
        {
            let mut tokens = DOWNLOAD_TOKENS.lock().await;
            tokens.insert(url_clone.clone(), cancellation_token.clone());
        }
        let url_clone_for_cleanup = url_clone.clone();
        let cleanup_token = cancellation_token.clone();
        let app_handle_clone_for_cleanup = app_handle_clone.clone();
        tokio::spawn(async move {
            cleanup_token.cancelled().await;
            let mut tokens = DOWNLOAD_TOKENS.lock().await;
            tokens.remove(&url_clone_for_cleanup);
            let _ = app_handle_clone_for_cleanup.emit("download-status", json!({
                "url": url_clone_for_cleanup,
                "status": "cancelled",
                "progress": 0,
            }));
        });


        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.expect("Failed to acquire semaphore permit");

            let download_result = if cancellation_token.is_cancelled() {
                println!("Download cancelled before starting: {}", url_clone);
                Err("Download cancelled".to_string())
            } else {
                download_textbook_internal(app_handle_clone, textbook_info_clone, token_clone, download_path_clone, cancellation_token).await
            };
            let mut tokens = DOWNLOAD_TOKENS.lock().await;
            tokens.remove(&url_clone);

            download_result
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
            },
            Err(join_err) => {
                eprintln!("Task join error: {}", join_err);
                all_succeeded = false;
            }
        }
    }
    if all_succeeded {
        println!("All batch downloads finished successfully.");
        let _ = app_handle.emit("batch-download-completed", json!({ "downloadPath": download_path }));
    } else {
        eprintln!("Some batch downloads failed.");
        let _ = app_handle.emit("batch-download-failed", json!({ "downloadPath": download_path }));
    }

    Ok(())
}