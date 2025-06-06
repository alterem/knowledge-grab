use tauri::Manager;

pub mod api;
pub mod downloader;
pub mod models;

pub use api::{fetch_filter_options, fetch_image, fetch_textbook_categories, fetch_textbooks};
pub use downloader::{batch_download_textbooks, cancel_download, download_textbook};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub debug_logging: bool,
    pub log_level: log::LevelFilter,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            debug_logging: cfg!(debug_assertions),
            log_level: if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            },
        }
    }
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::default();

    // 设置日志系统
    if config.debug_logging {
        app.handle()
            .plugin(
                tauri_plugin_log::Builder::default()
                    .level(config.log_level)
                    .build(),
            )
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        log::info!("应用程序启动 - 调试模式");
        log::debug!("日志级别: {:?}", config.log_level);
    }

    // 存储配置到应用状态
    app.manage(config);

    log::info!("应用程序初始化完成");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .setup(setup_app)
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("应用程序运行失败: {}", error);
        std::process::exit(1);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("网络请求失败: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON 解析失败: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("文件操作失败: {0}")]
    IoError(#[from] std::io::Error),

    #[error("URL 解析失败: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("下载被取消")]
    DownloadCancelled,

    #[error("参数验证失败: {0}")]
    ValidationError(String),

    #[error("应用程序错误: {0}")]
    Generic(String),
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();

        #[cfg(debug_assertions)]
        {
            assert!(config.debug_logging);
            assert_eq!(config.log_level, log::LevelFilter::Debug);
        }

        #[cfg(not(debug_assertions))]
        {
            assert!(!config.debug_logging);
            assert_eq!(config.log_level, log::LevelFilter::Info);
        }
    }

    #[test]
    fn test_app_error_conversion() {
        let error = AppError::ValidationError("测试错误".to_string());
        let error_string: String = error.into();
        assert!(error_string.contains("参数验证失败"));
    }
}
