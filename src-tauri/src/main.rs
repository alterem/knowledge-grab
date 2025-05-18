use tauri::{Manager, LogicalSize};
use tauri_plugin_fs::init as fs_init;
use tauri_plugin_dialog::init as dialog_init;

use std::sync::Mutex;

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub struct AppState {
    pub cached_category_id: Mutex<Option<String>>,
}

#[tauri::command]
async fn open_download_folder_prompt(download_path: String) -> Result<(), String> {
    println!("Opening download folder: {}", download_path);

    let path = std::path::Path::new(&download_path);
    if !path.exists() {
        return Err(format!("Download path does not exist: {}", download_path));
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer")
            .args([&download_path])
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .args([&download_path])
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let result = Command::new("xdg-open")
            .args([&download_path])
            .spawn();

        if result.is_err() {
            let result = Command::new("nautilus")
                .args([&download_path])
                .spawn();

            if result.is_err() {
                let result = Command::new("dolphin")
                    .args([&download_path])
                    .spawn();

                if result.is_err() {
                    let result = Command::new("thunar")
                        .args([&download_path])
                        .spawn();

                    if result.is_err() {
                        return Err("Failed to open folder: No suitable file manager found".to_string());
                    }
                }
            }
        }
    }

    Ok(())
}


fn main() {
    tauri::Builder::default()
        .plugin(fs_init())
        .plugin(dialog_init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                let main_window = app.get_webview_window("main").expect("failed to get main window");
                main_window.set_min_size(Some(LogicalSize::new(800.0, 600.0)))
                    .expect("failed to set min window size");
            }
            app.manage(AppState {
                cached_category_id: Mutex::new(None),
            });

            Ok(())
        })
        .on_menu_event(|_window, event| {
            println!("菜单项被点击: ID = {:?}", event.id);
            match event.id.as_ref() {
                "about" | "About" | "tray:about" | "macos:about" => {
                    println!("关于菜单被点击");
                    #[cfg(target_os = "macos")]
                    {
                        use std::process::Command;
                        let _ = Command::new("osascript")
                            .args(["-e", r#"display dialog "教科书下载器 v1.0.0\n\n© 2025 版权所有\n\n一个用于下载教科书的应用。" with title "关于教科书下载器" buttons {"确定"} default button "确定""#])
                            .spawn();
                    }

                    #[cfg(target_os = "windows")]
                    {
                        use std::process::Command;
                        let _ = Command::new("cmd")
                            .args(["/C", "start", "mshta", "javascript:alert('教科书下载器 v1.0.0\\n\\n© 2025 版权所有\\n\\n一个用于下载教科书的应用。');close()"])
                            .spawn();
                    }

                    #[cfg(target_os = "linux")]
                    {
                        use std::process::Command;
                        let _ = Command::new("zenity")
                            .args(["--info", "--title=关于教科书下载器", "--text=教科书下载器 v1.0.0\n\n© 2025 版权所有\n\n一个用于下载教科书的应用。"])
                            .spawn();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_lib::downloader::download_textbook,
            app_lib::downloader::batch_download_textbooks,
            app_lib::downloader::cancel_download,
            app_lib::api::fetch_textbooks,
            app_lib::api::fetch_filter_options,
            app_lib::api::fetch_textbook_categories,
            app_lib::api::fetch_image,
            open_download_folder_prompt
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
