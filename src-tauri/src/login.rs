use serde_json::json;
use tauri::{window::Color, Emitter, Manager, Theme, WebviewUrl, WebviewWindowBuilder};

const LOGIN_WINDOW_LABEL: &str = "smartedu-login";
// 中小学子平台：其鉴权 SDK 会把下载所需的 token 写入该域的 localStorage
const LOGIN_URL: &str = "https://basic.smartedu.cn/";
// 注入脚本拿到 token 后「跳转」到这个假域名，on_navigation 拦截并取消导航，
// 请求不会真正发出，仅作进程内消息通道
const TOKEN_CALLBACK_HOST: &str = "smartedu-token.callback";

// 与前端暗色主题的画布色一致（style.css 中 html.dark 的 --bg-color），
// 避免暗色模式下登录窗口在加载/导航间隙闪白底
const DARK_BG: Color = Color(0x14, 0x15, 0x18, 0xff);
const LIGHT_BG: Color = Color(0xff, 0xff, 0xff, 0xff);

pub const TOKEN_CAPTURED_EVENT: &str = "access-token-captured";

// 轮询 localStorage，等平台写入未过期的 "ND_UC_AUTH-...&token" 后回传 access_token 与 mac_key
// （二者是 entry.value 里的兄弟字段；mac_key 供视频课件的 doc-center 签名下载使用）
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
      var value = JSON.parse(entry.value);
      var token = value.access_token;
      var macKey = value.mac_key || "";
      if (token) {
        clearInterval(timer);
        window.location.href = "https://smartedu-token.callback/#token=" +
          encodeURIComponent(token) + "&mac_key=" + encodeURIComponent(macKey);
      }
    } catch (e) { /* 尚未登录，继续轮询 */ }
  }, 1000);
})();
"#;

/// 打开官方平台登录窗口；登录成功后注入脚本捕获 token，
/// 通过 access-token-captured 事件回传前端并自动关窗。用户凭据不经过本应用。
/// 必须是同步命令：macOS 上创建 webview 窗口要求在主线程执行。
/// is_dark 来自前端当前主题，用于让窗口主题与背景色跟随应用。
#[tauri::command]
pub fn open_login_window(app_handle: tauri::AppHandle, is_dark: Option<bool>) -> Result<(), String> {
    let is_dark = is_dark.unwrap_or(false);
    let theme = if is_dark { Theme::Dark } else { Theme::Light };
    let background = if is_dark { DARK_BG } else { LIGHT_BG };

    // 已有登录窗口时聚焦即可，避免叠开；顺带同步一次主题
    if let Some(existing) = app_handle.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = existing.set_theme(Some(theme));
        let _ = existing.set_background_color(Some(background));
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
    .theme(Some(theme))
    .background_color(background)
    .initialization_script(TOKEN_POLL_SCRIPT)
    .on_navigation(move |url| {
        if url.host_str() != Some(TOKEN_CALLBACK_HOST) {
            return true;
        }

        // fragment 形如 "token=xxx&mac_key=yyy"，逐对百分号解码
        let (mut token, mut mac_key) = (String::new(), String::new());
        if let Some(fragment) = url.fragment() {
            for pair in fragment.split('&') {
                let Some((k, v)) = pair.split_once('=') else {
                    continue;
                };
                let decoded = percent_encoding::percent_decode_str(v)
                    .decode_utf8_lossy()
                    .into_owned();
                match k {
                    "token" => token = decoded,
                    "mac_key" => mac_key = decoded,
                    _ => {}
                }
            }
        }

        // 在导航回调外发事件并关窗，避免 webview 导航中的重入问题
        let app = nav_handle.clone();
        tauri::async_runtime::spawn(async move {
            if !token.is_empty() {
                log::info!("已从登录窗口捕获 Access Token");
                let _ = app.emit(
                    TOKEN_CAPTURED_EVENT,
                    json!({ "token": token, "mac_key": mac_key }),
                );
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
