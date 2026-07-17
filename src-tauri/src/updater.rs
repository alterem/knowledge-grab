// 应用内更新：用 Rust 侧 updater API 而非插件的 JS 命令，
// 因为只有 Rust 侧能在运行时覆盖更新源（设置页的「自定义更新源/镜像」）。
// check_update 找到的更新对象暂存于进程内，随后由 install_update 消费。

use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

static PENDING_UPDATE: Lazy<Mutex<Option<Update>>> = Lazy::new(|| Mutex::new(None));

#[derive(Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// 检查更新。custom_endpoint 非空时覆盖 tauri.conf.json 里的默认更新源
/// （值为完整的 latest.json 地址，供 GitHub 访问受限时走镜像）。
/// 返回 None 表示已是最新版本。
#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    custom_endpoint: Option<String>,
) -> Result<Option<UpdateInfo>, String> {
    let mut builder = app.updater_builder();

    if let Some(endpoint) = custom_endpoint
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        let url = endpoint
            .parse()
            .map_err(|e| format!("自定义更新源地址无效: {e}"))?;
        builder = builder
            .endpoints(vec![url])
            .map_err(|e| format!("自定义更新源地址无效: {e}"))?;
    }

    let updater = builder.build().map_err(|e| format!("初始化更新器失败: {e}"))?;
    let update = updater.check().await.map_err(|e| e.to_string())?;

    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        notes: u.body.clone(),
    });
    *PENDING_UPDATE.lock().await = update;
    Ok(info)
}

/// 下载并安装 check_update 找到的更新。进度通过 updater-progress 事件上报
/// （payload: { downloaded, total?, finished? }）。Windows 上安装器会自动退出应用；
/// macOS/Linux 安装完成后由前端调用 process 插件 relaunch。
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let update = PENDING_UPDATE
        .lock()
        .await
        .take()
        .ok_or("尚未检查到可用更新，请先检查更新")?;

    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit(
                    "updater-progress",
                    serde_json::json!({ "downloaded": downloaded, "total": total }),
                );
            },
            || {
                let _ = app.emit("updater-progress", serde_json::json!({ "finished": true }));
            },
        )
        .await
        .map_err(|e| format!("下载安装更新失败: {e}"))?;

    Ok(())
}
