use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{
    Emitter, Manager, Theme, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    webview::{NewWindowResponse, PageLoadEvent},
    window::Color,
};

const LOGIN_WINDOW_LABEL: &str = "smartedu-login";
// 直接打开平台自己的同源登录组件。相比先加载资源繁重的平台首页再点“登录”，
// 这条入口少一次 SPA 初始化和认证跳转，在系统 WebView 中稳定得多。
const LOGIN_URL: &str = "https://basic.smartedu.cn/uc?sdp-app-id=e8906b1f-786a-4cc1-9afc-17e39abd099d&show_header=false&show_wrapper=false&show_footer=false&auto_jump=false&sso=false&__client_id=O9b107EvSr98frNb48oI98KVr4Q17NrR#/login";
// 注入脚本拿到 token 后“跳转”到这个假域名，on_navigation 拦截并取消导航，
// 请求不会真正发出，仅作进程内消息通道。
const CALLBACK_HOST: &str = "smartedu-token.callback";

// 与前端暗色主题的画布色一致（style.css 中 html.dark 的 --bg-color），
// 避免暗色模式下登录窗口在加载/导航间隙闪白底
const DARK_BG: Color = Color(0x14, 0x15, 0x18, 0xff);
const LIGHT_BG: Color = Color(0xff, 0xff, 0xff, 0xff);

pub const TOKEN_CAPTURED_EVENT: &str = "access-token-captured";
pub const LOGIN_STATUS_EVENT: &str = "login-window-status";

fn emit_login_status(
    app: &tauri::AppHandle,
    status: &str,
    message: Option<&str>,
    url: Option<&str>,
) {
    let _ = app.emit(
        LOGIN_STATUS_EVENT,
        json!({ "status": status, "message": message, "url": url }),
    );
}

// 轮询 localStorage，等平台写入未过期的 "ND_UC_AUTH-...&token" 后回传 access_token 与 mac_key。
// 同时给第三方页面加一个最小加载兜底，脚本资源异常时不再只剩一块无限期白屏。
const TOKEN_POLL_SCRIPT: &str = r##"
(function () {
  if (window.__KG_TOKEN_POLL__) { return; }
  window.__KG_TOKEN_POLL__ = true;

  function notify(params) {
    window.location.href = "https://smartedu-token.callback/#" + params;
  }

  function parseJson(value) {
    if (typeof value !== "string") { return value; }
    try { return JSON.parse(value); } catch (_) { return null; }
  }

  var timer = setInterval(function () {
    try {
      var key = Object.keys(localStorage).find(function (k) {
        return k.indexOf("ND_UC_AUTH") === 0 && k.lastIndexOf("&token") === k.length - "&token".length;
      });
      if (!key) { return; }
      var entry = parseJson(localStorage.getItem(key));
      if (!entry) { return; }
      var expire = Number(entry.expire || entry.expires_at || 0);
      if (expire && expire < 1000000000000) { expire *= 1000; }
      if (expire && expire < Date.now() + 60000) { return; }
      var value = parseJson(entry.value) || entry;
      var token = value.access_token;
      var macKey = value.mac_key || "";
      if (token) {
        clearInterval(timer);
        notify("token=" + encodeURIComponent(token) + "&mac_key=" + encodeURIComponent(macKey));
      }
    } catch (_) { /* 尚未登录或存储正在更新，继续轮询 */ }
  }, 500);

  function hasLoginContent() {
    var root = document.getElementById("root-x-edu-web");
    if (root && root.childElementCount > 0) { return true; }
    return !!document.querySelector('input:not([type="hidden"]), form, button, [role="button"]');
  }

  function installLoadingNotice() {
    if (!document.body || document.getElementById("kg-login-loading")) { return; }
    if (hasLoginContent()) {
      notify("status=ready");
      return;
    }

    var notice = document.createElement("div");
    notice.id = "kg-login-loading";
    notice.style.cssText = "position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;flex-direction:column;gap:12px;background:#fff;color:#303133;font:14px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif";
    notice.innerHTML = '<strong style="font-size:16px">正在加载官方登录页面</strong><span id="kg-login-loading-detail" style="color:#73767a">请稍候...</span>';
    document.body.appendChild(notice);

    var checks = 0;
    var contentTimer = setInterval(function () {
      checks += 1;
      if (hasLoginContent()) {
        clearInterval(contentTimer);
        notice.remove();
        notify("status=ready");
        return;
      }
      if (checks === 40) {
        var detail = document.getElementById("kg-login-loading-detail");
        if (detail) {
          detail.textContent = "页面加载超时，请检查网络后关闭窗口重试";
          detail.style.color = "#c45656";
        }
      }
    }, 500);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
      setTimeout(installLoadingNotice, 800);
    }, { once: true });
  } else {
    setTimeout(installLoadingNotice, 800);
  }
})();
"##;

/// 打开官方平台登录窗口；登录成功后注入脚本捕获 token，
/// 通过 access-token-captured 事件回传前端并自动关窗。用户凭据不经过本应用。
/// 必须是异步命令：Windows 上在同步命令（运行于主线程）中创建 WebView2 窗口会死锁，
/// 表现为登录窗口黑屏、主窗口与登录窗口均无响应且无法关闭（#11）。
/// 异步命令运行在后台线程，Tauri 会把窗口创建派发到主线程事件循环，macOS 同样安全。
/// is_dark 来自前端当前主题，用于让窗口主题与背景色跟随应用。
#[tauri::command]
pub async fn open_login_window(
    app_handle: tauri::AppHandle,
    is_dark: Option<bool>,
) -> Result<(), String> {
    let is_dark = is_dark.unwrap_or(false);
    let theme = if is_dark { Theme::Dark } else { Theme::Light };
    let background = if is_dark { DARK_BG } else { LIGHT_BG };

    let login_url: url::Url = LOGIN_URL
        .parse()
        .map_err(|e| format!("无效的登录地址: {e}"))?;

    // 已有登录窗口时重新导航，既避免叠开，也让加载失败后的再次点击成为真正的重试。
    if let Some(existing) = app_handle.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = existing.set_theme(Some(theme));
        let _ = existing.set_background_color(Some(background));
        existing
            .navigate(login_url)
            .map_err(|e| format!("重新加载登录窗口失败: {e}"))?;
        let _ = existing.set_focus();
        emit_login_status(&app_handle, "loading", None, Some(LOGIN_URL));
        return Ok(());
    }

    let captured = Arc::new(AtomicBool::new(false));
    let page_ready = Arc::new(AtomicBool::new(false));
    let nav_handle = app_handle.clone();
    let nav_captured = captured.clone();
    let nav_ready = page_ready.clone();
    let load_handle = app_handle.clone();
    let popup_handle = app_handle.clone();

    let window = WebviewWindowBuilder::new(
        &app_handle,
        LOGIN_WINDOW_LABEL,
        WebviewUrl::External(login_url),
    )
    .title("登录国家中小学智慧教育平台")
    .inner_size(1100.0, 780.0)
    .theme(Some(theme))
    .background_color(background)
    .initialization_script(TOKEN_POLL_SCRIPT)
    .on_page_load(move |_window, payload| {
        let status = match payload.event() {
            PageLoadEvent::Started => "loading",
            PageLoadEvent::Finished => "loaded",
        };
        emit_login_status(&load_handle, status, None, Some(payload.url().as_str()));
    })
    .on_new_window(move |url, _features| {
        // 官方认证偶尔通过 window.open 打开统一登录页。子 WebView 不会继承本窗口的
        // Token 捕获脚本，因此把 http(s) 跳转继续放在当前登录窗口中。
        if matches!(url.scheme(), "http" | "https") {
            let app = popup_handle.clone();
            tauri::async_runtime::spawn(async move {
                emit_login_status(&app, "redirecting", None, Some(url.as_str()));
                if let Some(window) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                    let _ = window.navigate(url);
                }
            });
        }
        NewWindowResponse::Deny
    })
    .on_navigation(move |url| {
        if url.host_str() != Some(CALLBACK_HOST) {
            return true;
        }

        // fragment 形如 "token=xxx&mac_key=yyy" 或 "status=ready"。
        let (mut token, mut mac_key, mut status) = (String::new(), String::new(), String::new());
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
                    "status" => status = decoded,
                    _ => {}
                }
            }
        }

        if status == "ready" {
            nav_ready.store(true, Ordering::Release);
            emit_login_status(&nav_handle, "ready", None, None);
            return false;
        }

        if !token.is_empty() {
            nav_captured.store(true, Ordering::Release);
            let app = nav_handle.clone();
            tauri::async_runtime::spawn(async move {
                log::info!("已从登录窗口捕获 Access Token");
                let _ = app.emit(
                    TOKEN_CAPTURED_EVENT,
                    json!({ "token": token, "mac_key": mac_key }),
                );
                emit_login_status(&app, "succeeded", None, None);
                if let Some(window) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                    let _ = window.close();
                }
            });
        }

        false // 吞掉进程内回调导航，绝不能发到网络
    })
    .build()
    .map_err(|e| format!("打开登录窗口失败: {e}"))?;

    emit_login_status(&app_handle, "opened", None, Some(LOGIN_URL));

    let close_handle = app_handle.clone();
    let close_captured = captured.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) && !close_captured.load(Ordering::Acquire) {
            emit_login_status(
                &close_handle,
                "closed",
                Some("登录窗口已关闭，未获取到 Access Token"),
                None,
            );
        }
    });

    let timeout_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        if !captured.load(Ordering::Acquire)
            && !page_ready.load(Ordering::Acquire)
            && timeout_handle
                .get_webview_window(LOGIN_WINDOW_LABEL)
                .is_some()
        {
            emit_login_status(
                &timeout_handle,
                "timeout",
                Some("官方登录页加载超时，请检查网络后重试，或改用手动获取"),
                None,
            );
        }
    });

    Ok(())
}
