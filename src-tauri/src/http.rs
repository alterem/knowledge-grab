use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use std::time::Duration;

// 全局共享客户端：复用连接池；只设连接超时，下载大文件走流式不设总超时
pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("构建 HTTP 客户端失败")
});

async fn get_checked(url: &str) -> Result<reqwest::Response, String> {
    let response = CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求失败 {url}: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("请求失败 {url}: HTTP {status}"));
    }
    Ok(response)
}

pub async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    get_checked(url)
        .await?
        .json()
        .await
        .map_err(|e| format!("解析 JSON 失败 {url}: {e}"))
}

pub async fn get_bytes(url: &str) -> Result<bytes::Bytes, String> {
    get_checked(url)
        .await?
        .bytes()
        .await
        .map_err(|e| format!("读取响应失败 {url}: {e}"))
}
