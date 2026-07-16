use std::path::Path;

const APP_VERSION: &str = "1.0.0";
const APP_NAME: &str = "教科书下载器";
const COPYRIGHT: &str = "© 2025 版权所有";
const APP_DESCRIPTION: &str = "一个用于下载教科书的应用。";

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn spawn_command(program: &str, args: &[&str]) -> Result<(), String> {
    std::process::Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("无法执行 {program}: {e}"))
}

#[allow(unused_variables)]
fn open_folder(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("路径不存在: {path}"));
    }

    #[cfg(target_os = "windows")]
    spawn_command("explorer", &[path])?;

    #[cfg(target_os = "macos")]
    spawn_command("open", &[path])?;

    #[cfg(target_os = "linux")]
    {
        let opened = ["xdg-open", "nautilus", "dolphin", "thunar"]
            .iter()
            .any(|program| spawn_command(program, &[path]).is_ok());
        if !opened {
            return Err("未找到可用的文件管理器".to_string());
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn open_download_folder_prompt(download_path: String) -> Result<(), String> {
    open_folder(&download_path)
}

// 用系统默认程序打开文件（下载完成的视频/课件点击播放/查看）
#[allow(unused_variables)]
#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err(format!("文件不存在: {path}"));
    }

    #[cfg(target_os = "windows")]
    spawn_command("cmd", &["/C", "start", "", &path])?;

    #[cfg(target_os = "macos")]
    spawn_command("open", &[&path])?;

    #[cfg(target_os = "linux")]
    spawn_command("xdg-open", &[&path])?;

    Ok(())
}

// 在系统文件管理器中定位并选中文件（找不到文件时退回打开其所在目录）
#[allow(unused_variables)]
#[tauri::command]
pub async fn reveal_file(path: String) -> Result<(), String> {
    let file = Path::new(&path);
    if !file.exists() {
        // 文件已被删/移走时，退回打开父目录
        if let Some(parent) = file.parent().map(|p| p.to_string_lossy().into_owned()) {
            return open_folder(&parent);
        }
        return Err(format!("文件不存在: {path}"));
    }

    #[cfg(target_os = "windows")]
    spawn_command("explorer", &["/select,", &path])?;

    #[cfg(target_os = "macos")]
    spawn_command("open", &["-R", &path])?;

    #[cfg(target_os = "linux")]
    {
        // 多数 Linux 文件管理器不支持"选中"，直接打开父目录
        let parent = file
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .ok_or("无法确定父目录")?;
        open_folder(&parent)?;
    }

    Ok(())
}

#[allow(unused_variables)]
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    spawn_command("cmd", &["/C", "start", &url])?;

    #[cfg(target_os = "macos")]
    spawn_command("open", &[&url])?;

    #[cfg(target_os = "linux")]
    spawn_command("xdg-open", &[&url])?;

    Ok(())
}

pub fn show_about_dialog() -> Result<(), String> {
    #[allow(unused_variables)]
    let message = format!("{APP_NAME} v{APP_VERSION}\n\n{COPYRIGHT}\n\n{APP_DESCRIPTION}");

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"display dialog "{}" with title "关于{APP_NAME}" buttons {{"确定"}} default button "确定""#,
            message.replace('\n', "\\n"),
        );
        spawn_command("osascript", &["-e", &script])?;
    }

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "javascript:alert('{}');close()",
            message.replace('\n', "\\n")
        );
        spawn_command("cmd", &["/C", "start", "mshta", &script])?;
    }

    #[cfg(target_os = "linux")]
    {
        spawn_command(
            "zenity",
            &[
                "--info",
                &format!("--title=关于{APP_NAME}"),
                &format!("--text={message}"),
            ],
        )?;
    }

    Ok(())
}
