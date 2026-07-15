use serde_json::json;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const LOGIN_WINDOW_LABEL: &str = "smartedu-login";
// 中小学子平台：其鉴权 SDK 会把下载所需的 token 写入该域的 localStorage
const LOGIN_URL: &str = "https://basic.smartedu.cn/";
// 注入脚本拿到 token 后「跳转」到这个假域名，on_navigation 拦截并取消导航，
// 请求不会真正发出，仅作进程内消息通道
const TOKEN_CALLBACK_HOST: &str = "smartedu-token.callback";

pub const TOKEN_CAPTURED_EVENT: &str = "access-token-captured";

// 轮询 localStorage，等平台写入未过期的 "ND_UC_AUTH-...&token" 后回传 access_token
const TOKEN_POLL_SCRIPT: &str = r#"
(function () {
  if (window.__KG_TOKEN_POLL__) { return; }
  window.__KG_TOKEN_POLL__ = true;
  var timer = setInterval(function () {
    try {
      var key = Object.keys(localStorage).find(function (k) {
        return k.indexOf("ND_UC_AUTH") === 0 && k.lastIndexOf("&token") === k.length - "&token".length;
      });
      if (!key) { return; }
      var entry = JSON.parse(localStorage.getItem(key));
      // 跳过上次登录残留的过期令牌，等待平台刷新
      if (entry.expire && entry.expire < Date.now() + 60000) { return; }
      var token = JSON.parse(entry.value).access_token;
      if (token) {
        clearInterval(timer);
        window.location.href = "https://smartedu-token.callback/#" + encodeURIComponent(token);
      }
    } catch (e) { /* 尚未登录，继续轮询 */ }
  }, 1000);
})();
"#;

/// 打开官方平台登录窗口；登录成功后注入脚本捕获 token，
/// 通过 access-token-captured 事件回传前端并自动关窗。用户凭据不经过本应用。
/// 必须是同步命令：macOS 上创建 webview 窗口要求在主线程执行。
#[tauri::command]
pub fn open_login_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    // 已有登录窗口时聚焦即可，避免叠开
    if let Some(existing) = app_handle.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = existing.set_focus();
        return Ok(());
    }

    let login_url: url::Url = LOGIN_URL
        .parse()
        .map_err(|e| format!("无效的登录地址: {e}"))?;

    let nav_handle = app_handle.clone();
    WebviewWindowBuilder::new(
        &app_handle,
        LOGIN_WINDOW_LABEL,
        WebviewUrl::External(login_url),
    )
    .title("请在页面中完成登录（右上角），成功后本窗口将自动关闭")
    .inner_size(1100.0, 780.0)
    .initialization_script(TOKEN_POLL_SCRIPT)
    .on_navigation(move |url| {
        if url.host_str() != Some(TOKEN_CALLBACK_HOST) {
            return true;
        }

        let token = url
            .fragment()
            .map(|fragment| {
                percent_encoding::percent_decode_str(fragment)
                    .decode_utf8_lossy()
                    .into_owned()
            })
            .unwrap_or_default();

        // 在导航回调外发事件并关窗，避免 webview 导航中的重入问题
        let app = nav_handle.clone();
        tauri::async_runtime::spawn(async move {
            if !token.is_empty() {
                log::info!("已从登录窗口捕获 Access Token");
                let _ = app.emit(TOKEN_CAPTURED_EVENT, json!({ "token": token }));
            }
            if let Some(window) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                let _ = window.close();
            }
        });

        false // 吞掉回调导航，绝不能发到网络
    })
    .build()
    .map_err(|e| format!("打开登录窗口失败: {e}"))?;

    Ok(())
}
