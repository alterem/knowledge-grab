use serde_json::json;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const LOGIN_WINDOW_LABEL: &str = "smartedu-login";
// The 中小学 sub-platform: its auth SDK writes the ncet-xedu token (the one the
// ndr-private download hosts accept) into this origin's localStorage.
const LOGIN_URL: &str = "https://basic.smartedu.cn/";
// Fake host the injected script "navigates" to once the token appears. The
// on_navigation handler intercepts and blocks it, so no request ever leaves
// the app — it is purely an in-process message channel.
const TOKEN_CALLBACK_HOST: &str = "smartedu-token.callback";

pub const TOKEN_CAPTURED_EVENT: &str = "access-token-captured";

// Runs on every page of the login window. Polls localStorage until the
// platform writes a fresh (non-expired) "ND_UC_AUTH-...&token" entry, then
// hands the access_token to Rust via the fake callback navigation.
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
      // Skip stale tokens left over from a previous login: wait for the
      // platform to refresh the entry after the user signs in again.
      if (entry.expire && entry.expire < Date.now() + 60000) { return; }
      var token = JSON.parse(entry.value).access_token;
      if (token) {
        clearInterval(timer);
        window.location.href = "https://smartedu-token.callback/#" + encodeURIComponent(token);
      }
    } catch (e) { /* not logged in yet; keep polling */ }
  }, 1000);
})();
"#;

/// Opens a login window on the official platform page. Once the user signs in,
/// the injected script captures the access token and this command's navigation
/// handler forwards it to the frontend via the `access-token-captured` event,
/// then closes the window. The user's credentials never pass through the app.
///
/// Deliberately a sync command: it runs on the main thread, which webview
/// window creation requires on macOS.
#[tauri::command]
pub fn open_login_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Focus the existing login window instead of stacking a second one.
    if let Some(existing) = app_handle.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = existing.set_focus();
        return Ok(());
    }

    let login_url: url::Url = LOGIN_URL
        .parse()
        .map_err(|e| format!("Invalid login URL: {}", e))?;

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

        // Emit and close outside the navigation callback to avoid re-entrancy
        // into the webview while it is mid-navigation.
        let app = nav_handle.clone();
        tauri::async_runtime::spawn(async move {
            if !token.is_empty() {
                println!("Access token captured from login window");
                let _ = app.emit(TOKEN_CAPTURED_EVENT, json!({ "token": token }));
            }
            if let Some(window) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                let _ = window.close();
            }
        });

        false // swallow the callback navigation; it must never hit the network
    })
    .build()
    .map_err(|e| format!("Failed to open login window: {}", e))?;

    Ok(())
}
