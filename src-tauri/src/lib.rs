pub mod api;
pub mod downloader;
pub mod http;
pub mod login;
pub mod models;
pub mod system;

#[cfg(desktop)]
const MIN_WINDOW_WIDTH: f64 = 800.0;
#[cfg(desktop)]
const MIN_WINDOW_HEIGHT: f64 = 600.0;

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(desktop)]
    {
        use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    env_logger::init();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup_app)
        .on_menu_event(|_window, event| handle_menu_event(event.id.as_ref()))
        .invoke_handler(tauri::generate_handler![
            downloader::download_textbook,
            downloader::batch_download_textbooks,
            downloader::cancel_download,
            api::fetch_textbooks,
            api::fetch_filter_options,
            api::fetch_textbook_categories,
            api::fetch_image,
            api::clear_tch_material_tag_cache,
            login::open_login_window,
            system::open_download_folder_prompt,
            system::open_url
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("应用程序运行失败: {error}");
        std::process::exit(1);
    }
}
