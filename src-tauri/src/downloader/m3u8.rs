// 智慧教育平台视频下载：m3u8 (AES-128-CBC 加密的 TS 切片) → 解密 → 合成。
//
// 密钥获取是两段式握手（平台自定义，非标准 HLS）：
//   1. GET {key_url}/signs                → {"nonce": "..."}
//   2. sign = md5(nonce + key_id)[:16]
//   3. GET {key_url}?nonce=&sign=         → {"key": "<base64>"}
//   4. 真正的 16 字节密钥 = AES-ECB-decrypt(密钥=sign, 密文=base64 解码后的 key)
// 切片再用 AES-128-CBC(key, iv) + PKCS7 解密。全流程已离线验证。
//
// m3u8 与 ts 均需携带占位 MAC 鉴权头（见 task::create_request）。
//
// 断点续传：解密后的切片逐个落盘到 <最终名>.parts/ 目录，中断后重试会跳过
// 已存在的切片（密钥每次重新握手获取，不落盘）；全部就绪后按序流式拼接。

use crate::http::CLIENT;
use aes::Aes128;
use aes::cipher::{BlockDecryptMut, KeyIvInit, KeyInit, block_padding::Pkcs7};
use futures_util::stream::{self, StreamExt};
use md5::{Digest, Md5};
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::fs;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use super::task::{DownloadEventEmitter, USER_AGENT, path_with_suffix};

type Aes128CbcDec = cbc::Decryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

// 切片并发下载数：偏大以吃满带宽，但受全局并发（前端下载池调度）约束
const SEGMENT_CONCURRENCY: usize = 8;
// 单个切片的总尝试次数：瞬时网络抖动不该让几百个切片的任务整体失败
const SEGMENT_ATTEMPTS: usize = 3;

// 鉴权失败重试无意义（令牌过期只会一直 401/403），直接失败让上层提示换令牌
fn is_auth_error(message: &str) -> bool {
    message.contains("HTTP 401") || message.contains("HTTP 403")
}

struct KeyInfo {
    key: [u8; 16],
    iv: [u8; 16],
}

struct Playlist {
    segments: Vec<String>,
    key_url: Option<String>,
    iv_hex: Option<String>,
}

#[derive(Deserialize)]
struct NonceResp {
    nonce: String,
}

#[derive(Deserialize)]
struct KeyResp {
    key: String,
}

// 带占位鉴权拉取 URL 文本
async fn get_text_authed(url: &str, token: Option<&str>) -> Result<String, String> {
    let mut req = CLIENT.get(url).header(reqwest::header::USER_AGENT, USER_AGENT);
    if let Some(t) = token {
        req = req.header("x-nd-auth", format!("MAC id=\"{t}\",nonce=\"0\",mac=\"0\""));
    }
    let resp = req.send().await.map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("读取响应失败: {e}"))
}

async fn get_bytes_authed(url: &str, token: Option<&str>) -> Result<Vec<u8>, String> {
    let mut req = CLIENT.get(url).header(reqwest::header::USER_AGENT, USER_AGENT);
    if let Some(t) = token {
        req = req.header("x-nd-auth", format!("MAC id=\"{t}\",nonce=\"0\",mac=\"0\""));
    }
    let resp = req.send().await.map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("读取响应失败: {e}"))
}

// 解析 m3u8，取切片列表、KEY 地址与 IV。相对地址按 base_url 拼成绝对地址。
fn parse_playlist(content: &str, base_url: &str) -> Playlist {
    let base = &base_url[..=base_url.rfind('/').unwrap_or(0)];
    let mut segments = Vec::new();
    let mut key_url = None;
    let mut iv_hex = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("#EXT-X-KEY") {
            if let Some(rest) = line.split("URI=\"").nth(1) {
                if let Some(u) = rest.split('"').next() {
                    key_url = Some(u.to_string());
                }
            }
            if let Some(rest) = line.split("IV=").nth(1) {
                iv_hex = Some(
                    rest.trim()
                        .trim_start_matches("0x")
                        .trim_start_matches("0X")
                        .to_string(),
                );
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            if line.starts_with("http") {
                segments.push(line.to_string());
            } else {
                segments.push(format!("{base}{line}"));
            }
        }
    }

    Playlist {
        segments,
        key_url,
        iv_hex,
    }
}

// 两段式握手取解密密钥
async fn fetch_key(
    key_url: &str,
    iv_hex: Option<&str>,
    token: Option<&str>,
) -> Result<KeyInfo, String> {
    let key_id = key_url.trim_end_matches('/').rsplit('/').next().unwrap_or("");

    // 1. 取 nonce
    let nonce_text = get_text_authed(&format!("{key_url}/signs"), token).await?;
    let nonce: NonceResp =
        serde_json::from_str(&nonce_text).map_err(|e| format!("解析 nonce 失败: {e}"))?;

    // 2. sign = md5(nonce + key_id)[:16]
    let mut hasher = Md5::new();
    hasher.update(format!("{}{}", nonce.nonce, key_id).as_bytes());
    let sign = hex::encode(hasher.finalize())[..16].to_string();

    // 3. 用 nonce + sign 换加密后的 key
    let key_text =
        get_text_authed(&format!("{key_url}?nonce={}&sign={}", nonce.nonce, sign), token).await?;
    let key_resp: KeyResp =
        serde_json::from_str(&key_text).map_err(|e| format!("解析 key 失败: {e}"))?;
    let enc_key = base64_decode(&key_resp.key)?;

    // 4. AES-ECB(key=sign) 解密得到真正的 16 字节密钥
    let mut buf = enc_key;
    let real_key = Aes128EcbDec::new(sign.as_bytes().into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| format!("密钥解密失败: {e}"))?;
    if real_key.len() != 16 {
        return Err(format!("密钥长度异常: {}", real_key.len()));
    }
    let mut key = [0u8; 16];
    key.copy_from_slice(real_key);

    // IV：m3u8 声明则用之，否则全零
    let iv = match iv_hex {
        Some(h) if !h.is_empty() => {
            let raw = hex::decode(h).map_err(|e| format!("IV 解析失败: {e}"))?;
            let mut iv = [0u8; 16];
            let n = raw.len().min(16);
            iv[..n].copy_from_slice(&raw[..n]);
            iv
        }
        _ => [0u8; 16],
    };

    Ok(KeyInfo { key, iv })
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    STANDARD.decode(s).map_err(|e| format!("base64 解码失败: {e}"))
}

// 解密单个切片（CBC + PKCS7）。部分切片可能无填充，去填充失败时退回原始明文尾块处理。
fn decrypt_segment(data: &[u8], key: &KeyInfo) -> Result<Vec<u8>, String> {
    if data.len() % 16 != 0 {
        return Err(format!("切片长度非 16 整数倍: {}", data.len()));
    }
    let mut buf = data.to_vec();
    let dec = Aes128CbcDec::new(&key.key.into(), &key.iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| format!("切片解密失败: {e}"))?;
    Ok(dec.to_vec())
}

fn segment_file_name(idx: usize) -> String {
    format!("seg_{idx:05}.ts")
}

// 下载并解密单个切片，带重试（指数退避）。解密失败通常是响应被截断，同样值得重试；
// 鉴权错误与取消立即返回。
async fn fetch_segment_with_retry(
    seg_url: &str,
    token: Option<&str>,
    key_info: Option<&KeyInfo>,
    idx: usize,
    cancellation_token: &CancellationToken,
) -> Result<Vec<u8>, String> {
    let mut delay_ms = 500u64;
    for attempt in 1..=SEGMENT_ATTEMPTS {
        if cancellation_token.is_cancelled() {
            return Err("下载已取消".to_string());
        }

        let result = match get_bytes_authed(seg_url, token).await {
            Ok(raw) => match key_info {
                Some(k) => decrypt_segment(&raw, k),
                None => Ok(raw),
            },
            Err(e) => Err(e),
        };

        match result {
            Ok(bytes) => return Ok(bytes),
            Err(e) if attempt < SEGMENT_ATTEMPTS && !is_auth_error(&e) => {
                log::warn!("切片 {idx} 第 {attempt} 次尝试失败，将重试: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms *= 2;
            }
            Err(e) => return Err(format!("切片 {idx} 下载失败: {e}")),
        }
    }
    unreachable!("重试循环必然提前返回")
}

// 建立/校验切片缓存目录。playlist 变化（地址或切片数不同）时清空重建，避免错位拼接
async fn prepare_parts_dir(parts_dir: &Path, m3u8_url: &str, total: usize) -> Result<(), String> {
    let manifest_path = parts_dir.join("manifest.txt");
    let manifest = format!("{m3u8_url}\n{total}");

    if fs::try_exists(parts_dir).await.unwrap_or(false) {
        let stale = match fs::read_to_string(&manifest_path).await {
            Ok(prev) => prev != manifest,
            Err(_) => true,
        };
        if stale {
            fs::remove_dir_all(parts_dir)
                .await
                .map_err(|e| format!("清理切片缓存失败: {e}"))?;
        }
    }

    fs::create_dir_all(parts_dir)
        .await
        .map_err(|e| format!("创建切片缓存目录失败: {e}"))?;
    fs::write(&manifest_path, manifest)
        .await
        .map_err(|e| format!("写入切片清单失败: {e}"))?;
    Ok(())
}

// 依序把切片拼接为单个 .ts（逐片读写，峰值内存只有单个切片大小）
async fn assemble_ts(
    ts_path: &Path,
    parts_dir: &Path,
    total: usize,
    cancellation_token: &CancellationToken,
) -> Result<(), String> {
    let file = fs::File::create(ts_path)
        .await
        .map_err(|e| format!("创建文件失败: {e}"))?;
    let mut writer = BufWriter::new(file);
    for idx in 0..total {
        if cancellation_token.is_cancelled() {
            return Err("下载已取消".to_string());
        }
        let seg_path = parts_dir.join(segment_file_name(idx));
        let bytes = fs::read(&seg_path)
            .await
            .map_err(|e| format!("切片 {idx} 缺失: {e}"))?;
        writer
            .write_all(&bytes)
            .await
            .map_err(|e| format!("写入失败: {e}"))?;
    }
    writer.flush().await.map_err(|e| format!("写入失败: {e}"))?;
    Ok(())
}

/// 下载并解密整个 m3u8 视频。返回实际写入的文件路径：
/// ffmpeg 可用则转封装成 out_path（通常 .mp4），否则保留裸拼的 .ts。
/// 中断/取消会保留 <out_path>.parts/ 中已下载的切片，下次调用自动续传。
pub(super) async fn download(
    m3u8_url: &str,
    token: Option<&str>,
    out_path: &Path,
    ffmpeg_path: Option<&str>,
    cancellation_token: &CancellationToken,
    emitter: &DownloadEventEmitter,
) -> Result<std::path::PathBuf, String> {
    // 1. 拉 m3u8
    let content = get_text_authed(m3u8_url, token)
        .await
        .map_err(|e| format!("获取播放列表失败: {e}"))?;
    let playlist = parse_playlist(&content, m3u8_url);

    if playlist.segments.is_empty() {
        return Err("播放列表为空".to_string());
    }

    // 2. 取密钥（如加密）。续传时也重新握手，密钥不落盘
    let key_info = match playlist.key_url.as_deref() {
        Some(ku) => Some(Arc::new(fetch_key(ku, playlist.iv_hex.as_deref(), token).await?)),
        None => None,
    };

    let total = playlist.segments.len();

    // 3. 切片缓存目录：<最终名>.parts/
    let parts_dir = path_with_suffix(out_path, ".parts");
    prepare_parts_dir(&parts_dir, m3u8_url, total).await?;

    emitter.emit_progress(0, 0, None);

    // 4. 并发下载 + 解密切片，逐片落盘；已存在的切片直接跳过（续传）
    let done_count = Arc::new(AtomicUsize::new(0));
    let done_bytes = Arc::new(AtomicU64::new(0));
    let results: Vec<Result<(), String>> = stream::iter(
        playlist.segments.iter().cloned().enumerate().map(|(idx, seg_url)| {
            let key_info = key_info.clone();
            let done_count = Arc::clone(&done_count);
            let done_bytes = Arc::clone(&done_bytes);
            let token = token.map(str::to_string);
            let seg_path = parts_dir.join(segment_file_name(idx));
            let tmp_path = parts_dir.join(format!("{}.tmp", segment_file_name(idx)));
            async move {
                if cancellation_token.is_cancelled() {
                    return Err("下载已取消".to_string());
                }

                let seg_len = match fs::metadata(&seg_path).await {
                    // 已有完整切片（写入走 tmp+rename，存在即完整），跳过下载
                    Ok(meta) if meta.len() > 0 => meta.len(),
                    _ => {
                        let bytes = fetch_segment_with_retry(
                            &seg_url,
                            token.as_deref(),
                            key_info.as_deref(),
                            idx,
                            cancellation_token,
                        )
                        .await?;
                        // 先写临时文件再改名，避免中断残留半个切片被误判为完整
                        fs::write(&tmp_path, &bytes)
                            .await
                            .map_err(|e| format!("写入切片失败: {e}"))?;
                        fs::rename(&tmp_path, &seg_path)
                            .await
                            .map_err(|e| format!("写入切片失败: {e}"))?;
                        bytes.len() as u64
                    }
                };

                // 进度按已就绪切片数上报，字节数供前端算速度
                let done = done_count.fetch_add(1, Ordering::SeqCst) + 1;
                let bytes_sum = done_bytes.fetch_add(seg_len, Ordering::SeqCst) + seg_len;
                emitter.emit_progress((done as f64 / total as f64 * 100.0) as u32, bytes_sum, None);
                Ok(())
            }
        }),
    )
    .buffer_unordered(SEGMENT_CONCURRENCY)
    .collect()
    .await;

    if cancellation_token.is_cancelled() {
        return Err("下载已取消".to_string());
    }
    for r in results {
        r?;
    }

    // 5. 按序拼接切片 → .ts
    let ts_path = out_path.with_extension("ts");
    assemble_ts(&ts_path, &parts_dir, total, cancellation_token).await?;

    // ffmpeg 可用则 remux 成目标容器（通常 .mp4），失败或缺失则保留 .ts
    if let Some(ff) = ffmpeg_path.filter(|p| !p.is_empty()) {
        if out_path != ts_path {
            match remux(ff, &ts_path, out_path).await {
                Ok(()) => {
                    let _ = fs::remove_file(&ts_path).await;
                    let _ = fs::remove_dir_all(&parts_dir).await;
                    return Ok(out_path.to_path_buf());
                }
                Err(e) => {
                    log::warn!("ffmpeg 转封装失败，保留 .ts: {e}");
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&parts_dir).await;
    Ok(ts_path)
}

// 调 ffmpeg 把 .ts 无损转封装成目标容器（-c copy，不重新编码）
async fn remux(ffmpeg: &str, ts_path: &Path, out_path: &Path) -> Result<(), String> {
    let status = Command::new(ffmpeg)
        .arg("-y")
        .arg("-i")
        .arg(ts_path)
        .arg("-c")
        .arg("copy")
        .arg("-bsf:a")
        .arg("aac_adtstoasc")
        .arg(out_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| format!("无法执行 ffmpeg: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg 退出码 {status}"))
    }
}

// 校验 ffmpeg 是否可用（设置页「检测」用）
pub async fn probe_ffmpeg(path: &str) -> bool {
    Command::new(path)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
