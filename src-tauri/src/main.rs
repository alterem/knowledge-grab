use tauri::{LogicalSize, Manager};
use tauri_plugin_dialog::init as dialog_init;
use tauri_plugin_fs::init as fs_init;

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[derive(Debug)]
pub struct AppState {
    pub cached_category_id: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cached_category_id: Mutex::new(None),
        }
    }

    pub fn get_cached_category_id(&self) -> Option<String> {
        self.cached_category_id.lock().unwrap().clone()
    }

    pub fn set_cached_category_id(&self, category_id: Option<String>) {
        *self.cached_category_id.lock().unwrap() = category_id;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

mod constants {
    pub const APP_VERSION: &str = "1.0.0";
    pub const APP_NAME: &str = "教科书下载器";
    pub const COPYRIGHT: &str = "© 2025 版权所有";
    pub const APP_DESCRIPTION: &str = "一个用于下载教科书的应用。";

    pub const MIN_WINDOW_WIDTH: f64 = 800.0;
    pub const MIN_WINDOW_HEIGHT: f64 = 600.0;
}

mod file_manager {
    use super::*;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum FileManagerType {
        Windows,
        MacOS,
        Linux(LinuxFileManager),
    }

    #[derive(Debug, Clone)]
    enum LinuxFileManager {
        XdgOpen,
        Nautilus,
        Dolphin,
        Thunar,
    }

    fn get_file_manager() -> FileManagerType {
        #[cfg(target_os = "windows")]
        return FileManagerType::Windows;

        #[cfg(target_os = "macos")]
        return FileManagerType::MacOS;

        #[cfg(target_os = "linux")]
        return FileManagerType::Linux(LinuxFileManager::XdgOpen);
    }

    fn try_linux_file_manager(path: &str, manager: LinuxFileManager) -> Result<(), std::io::Error> {
        let command = match manager {
            LinuxFileManager::XdgOpen => "xdg-open",
            LinuxFileManager::Nautilus => "nautilus",
            LinuxFileManager::Dolphin => "dolphin",
            LinuxFileManager::Thunar => "thunar",
        };

        Command::new(command).args([path]).spawn().map(|_| ())
    }

    pub fn open_folder(path: &str) -> Result<(), String> {
        log::info!("尝试打开文件夹: {}", path);

        if !Path::new(path).exists() {
            let error_msg = format!("路径不存在: {}", path);
            log::error!("{}", error_msg);
            return Err(error_msg);
        }

        match get_file_manager() {
            FileManagerType::Windows => {
                Command::new("explorer")
                    .args([path])
                    .spawn()
                    .map_err(|e| format!("无法打开文件夹: {}", e))?;
            }

            FileManagerType::MacOS => {
                Command::new("open")
                    .args([path])
                    .spawn()
                    .map_err(|e| format!("无法打开文件夹: {}", e))?;
            }

            FileManagerType::Linux(_) => {
                let managers = [
                    LinuxFileManager::XdgOpen,
                    LinuxFileManager::Nautilus,
                    LinuxFileManager::Dolphin,
                    LinuxFileManager::Thunar,
                ];

                let mut last_error = None;
                for manager in &managers {
                    match try_linux_file_manager(path, manager.clone()) {
                        Ok(()) => {
                            log::info!("成功使用 {:?} 打开文件夹", manager);
                            return Ok(());
                        }
                        Err(e) => {
                            log::debug!("尝试 {:?} 失败: {}", manager, e);
                            last_error = Some(e);
                        }
                    }
                }

                return Err(format!(
                    "无法找到合适的文件管理器。最后一个错误: {:?}",
                    last_error.unwrap_or_else(|| std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "未知错误"
                    ))
                ));
            }
        }

        log::info!("成功打开文件夹: {}", path);
        Ok(())
    }
}

mod system_dialog {
    use super::constants::*;
    use super::*;

    pub fn show_about_dialog() -> Result<(), String> {
        let message = format!(
            "{} v{}\n\n{}\n\n{}",
            APP_NAME, APP_VERSION, COPYRIGHT, APP_DESCRIPTION
        );

        log::info!("显示关于对话框");

        #[cfg(target_os = "macos")]
        {
            let script = format!(
                r#"display dialog "{}" with title "关于{}" buttons {{"确定"}} default button "确定""#,
                message.replace('\n', "\\n"),
                APP_NAME
            );

            Command::new("osascript")
                .args(["-e", &script])
                .spawn()
                .map_err(|e| format!("无法显示对话框: {}", e))?;
        }

        #[cfg(target_os = "windows")]
        {
            let escaped_message = message.replace('\n', "\\n");
            let script = format!("javascript:alert('{}');close()", escaped_message);

            Command::new("cmd")
                .args(["/C", "start", "mshta", &script])
                .spawn()
                .map_err(|e| format!("无法显示对话框: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("zenity")
                .args([
                    "--info",
                    &format!("--title=关于{}", APP_NAME),
                    &format!("--text={}", message),
                ])
                .spawn()
                .map_err(|e| format!("无法显示对话框: {}", e))?;
        }

        Ok(())
    }
}

#[tauri::command]
async fn open_download_folder_prompt(download_path: String) -> Result<(), String> {
    file_manager::open_folder(&download_path)
}

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    log::info!("尝试打开 URL: {}", url);

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("无法打开 URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args([&url])
            .spawn()
            .map_err(|e| format!("无法打开 URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .args([&url])
            .spawn()
            .map_err(|e| format!("无法打开 URL: {}", e))?;
    }

    log::info!("成功打开 URL: {}", url);
    Ok(())
}

fn setup_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(desktop)]
    {
        let main_window = app.get_webview_window("main").ok_or("无法获取主窗口")?;

        main_window
            .set_min_size(Some(LogicalSize::new(
                constants::MIN_WINDOW_WIDTH,
                constants::MIN_WINDOW_HEIGHT,
            )))
            .map_err(|e| format!("无法设置窗口最小尺寸: {}", e))?;

        log::info!(
            "窗口最小尺寸设置为: {}x{}",
            constants::MIN_WINDOW_WIDTH,
            constants::MIN_WINDOW_HEIGHT
        );
    }

    Ok(())
}

fn handle_menu_event(event_id: &str) {
    log::info!("菜单项被点击: ID = {}", event_id);

    match event_id {
        "about" | "About" | "tray:about" | "macos:about" => {
            log::info!("显示关于对话框");
            if let Err(e) = system_dialog::show_about_dialog() {
                log::error!("显示关于对话框失败: {}", e);
            }
        }
        "quit" | "Quit" | "tray:quit" => {
            log::info!("退出应用程序");
            std::process::exit(0);
        }
        _ => {
            log::debug!("未处理的菜单事件: {}", event_id);
        }
    }
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    setup_window(app)?;

    app.manage(AppState::new());

    log::info!("应用程序设置完成");
    Ok(())
}

fn main() {
    #[cfg(debug_assertions)]
    {
        env_logger::init();
        log::info!("应用程序启动 - 调试模式");
    }

    let result = tauri::Builder::default()
        .plugin(fs_init())
        .plugin(dialog_init())
        .setup(setup_app)
        .on_menu_event(|_window, event| {
            handle_menu_event(event.id.as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            app_lib::downloader::download_textbook,
            app_lib::downloader::batch_download_textbooks,
            app_lib::downloader::cancel_download,
            app_lib::api::fetch_textbooks,
            app_lib::api::fetch_filter_options,
            app_lib::api::fetch_textbook_categories,
            app_lib::api::fetch_image,
            open_download_folder_prompt,
            open_url
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("应用程序运行失败: {}", error);
        log::error!("应用程序运行失败: {}", error);
        std::process::exit(1);
    }
}
