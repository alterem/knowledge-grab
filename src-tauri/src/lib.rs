pub mod api;
pub mod downloader;
pub mod http;
pub mod login;
pub mod models;
pub mod system;
pub mod updater;

#[cfg(desktop)]
const MIN_WINDOW_WIDTH: f64 = 800.0;
#[cfg(desktop)]
const MIN_WINDOW_HEIGHT: f64 = 600.0;

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

        // updater / process 仅桌面端可用，在 setup 中按平台注册
        app.handle()
            .plugin(tauri_plugin_updater::Builder::new().build())?;
        app.handle().plugin(tauri_plugin_process::init())?;

        let window = app.get_webview_window("main").ok_or("无法获取主窗口")?;
        window.set_min_size(Some(tauri::LogicalSize::new(
            MIN_WINDOW_WIDTH,
            MIN_WINDOW_HEIGHT,
        )))?;
    }

    Ok(())
}

fn handle_menu_event(event_id: &str) {
    match event_id {
        "about" | "About" | "tray:about" | "macos:about" => {
            if let Err(e) = system::show_about_dialog() {
                log::error!("显示关于对话框失败: {e}");
            }
        }
        "quit" | "Quit" | "tray:quit" => std::process::exit(0),
        _ => {}
    }
}

// 把 panic 信息追加写入临时目录的崩溃日志（Windows: %TEMP%\knowledge-grab-crash.log）。
// release 构建没有控制台，没有这份日志的话闪退不留任何痕迹、无法定位原因。
fn install_crash_logger() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backtrace = std::backtrace::Backtrace::force_capture();
        let content = format!("[unix {timestamp}] 应用崩溃: {info}\n堆栈:\n{backtrace}\n\n");
        let path = std::env::temp_dir().join("knowledge-grab-crash.log");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, content.as_bytes()));
        default_hook(info);
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_crash_logger();

    #[cfg(debug_assertions)]
    env_logger::init();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup_app)
        .on_menu_event(|_window, event| handle_menu_event(event.id.as_ref()))
        .invoke_handler(tauri::generate_handler![
            downloader::download_textbook,
            downloader::cancel_download,
            downloader::download_course_resource,
            downloader::remove_download_artifacts,
            downloader::check_ffmpeg,
            api::fetch_textbooks,
            api::fetch_filter_options,
            api::fetch_textbook_categories,
            api::fetch_cover,
            api::fetch_image,
            api::courses::parse_course_url,
            api::clear_tch_material_tag_cache,
            login::open_login_window,
            system::open_download_folder_prompt,
            system::open_file,
            system::reveal_file,
            system::open_url,
            updater::check_update,
            updater::install_update
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("应用程序运行失败: {error}");
        std::process::exit(1);
    }
}
