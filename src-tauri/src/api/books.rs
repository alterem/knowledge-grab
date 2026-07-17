use crate::http;
use crate::models::{CustomProperties, DataVersion, RawBook, Textbook};
use futures_util::future::join_all;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const DATA_VERSION_URL: &str =
    "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/resources/tch_material/version/data_version.json";
const STATISTICS_URL: &str =
    "https://x-api.ykt.eduyun.cn/proxy/cloud/v1/res_stats/actions/query?res_ids=";
const RESOURCE_DETAIL_URL_PREFIX: &str =
    "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/resources/tch_material/details/";
// 教材 PDF 已从公开桶迁到鉴权私有桶，私有域名保留稳定的 pdf.pdf 别名，
// 但请求必须携带 x-nd-auth 头（见 downloader 的 create_request）
const DOWNLOAD_URL_PREFIX: &str = "https://r1-ndr-private.ykt.cbern.com.cn/edu_product/esp/assets/";
const DOWNLOAD_URL_SUFFIX: &str = ".pkg/pdf.pdf";

// 统计接口用查询串传 id，分批防止 URL 过长
const STATISTICS_BATCH_SIZE: usize = 50;

struct BooksCache {
    version: u64,
    books: Arc<Vec<RawBook>>,
}

// 全量书目有数 MB，按 module_version 缓存；tokio Mutex 同时把并发请求合并为一次下载
static BOOKS_CACHE: Lazy<Mutex<Option<BooksCache>>> = Lazy::new(|| Mutex::new(None));

pub async fn get_raw_books() -> Result<Arc<Vec<RawBook>>, String> {
    // data_version 是小文件，每次都拉取，用于感知目录更新
    let version: DataVersion = http::get_json(DATA_VERSION_URL).await?;

    let mut cache = BOOKS_CACHE.lock().await;
    if let Some(cached) = cache.as_ref() {
        if cached.version == version.module_version {
            return Ok(Arc::clone(&cached.books));
        }
    }

    log::info!(
        "拉取全量书目 (版本 {}): {}",
        version.module_version,
        version.urls
    );
    let books = Arc::new(fetch_raw_books(&version).await?);
    *cache = Some(BooksCache {
        version: version.module_version,
        books: Arc::clone(&books),
    });
    Ok(books)
}

pub async fn clear_cache() {
    *BOOKS_CACHE.lock().await = None;
    DETAIL_CACHE.lock().await.clear();
}

#[derive(Debug, Deserialize)]
pub struct ResourceDetail {
    #[serde(default)]
    ti_items: Vec<TiItem>,
}

#[derive(Debug, Deserialize)]
struct TiItem {
    #[serde(default)]
    ti_file_flag: Option<String>,
    #[serde(default)]
    ti_format: Option<String>,
    #[serde(default)]
    ti_storages: Vec<String>,
}

// 资源详情缓存：下载与封面共用，避免重复请求
static DETAIL_CACHE: Lazy<Mutex<HashMap<String, Arc<ResourceDetail>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

async fn get_detail(book_id: &str) -> Option<Arc<ResourceDetail>> {
    if let Some(detail) = DETAIL_CACHE.lock().await.get(book_id) {
        return Some(Arc::clone(detail));
    }

    let url = format!("{RESOURCE_DETAIL_URL_PREFIX}{book_id}.json");
    match http::get_json::<ResourceDetail>(&url).await {
        Ok(detail) => {
            let detail = Arc::new(detail);
            DETAIL_CACHE
                .lock()
                .await
                .insert(book_id.to_string(), Arc::clone(&detail));
            Some(detail)
        }
        Err(e) => {
            // thematic_course 等包装类型没有 tch_material 详情，属预期情况
            log::debug!("查询资源详情失败: {e}");
            None
        }
    }
}

// 从下载 URL 提取资源 id（…/assets/{id}.pkg/…）
pub fn resource_id_from_url(url: &str) -> Option<&str> {
    let rest = url.split("/assets/").nth(1)?;
    let id = rest.split(".pkg").next()?;
    (!id.is_empty()).then_some(id)
}

// 详情里声明的源 PDF 真实地址（多个 CDN 镜像）。
// 部分教材（如盲校/特教）不走电子书处理管线，pkg 里没有 pdf.pdf 别名，
// 只能按详情里声明的原始文件名下载；这也是官方阅读器的取法。
pub async fn resolve_source_pdf_urls(resource_id: &str) -> Vec<String> {
    let Some(detail) = get_detail(resource_id).await else {
        return Vec::new();
    };

    let source = detail
        .ti_items
        .iter()
        .find(|ti| {
            ti.ti_file_flag.as_deref() == Some("source") && ti.ti_format.as_deref() == Some("pdf")
        })
        .or_else(|| {
            detail
                .ti_items
                .iter()
                .find(|ti| ti.ti_format.as_deref() == Some("pdf"))
        });

    source.map(|ti| ti.ti_storages.clone()).unwrap_or_default()
}

// 封面取详情里的 thumbnail_1：它与源 PDF 同批转码生成，保证列表封面与下载文件一致。
// 详情存储在鉴权私有域名上，同路径的公开域名可免鉴权访问，替换后返回
pub async fn resolve_cover_url(book_id: &str) -> Option<String> {
    let detail = get_detail(book_id).await?;
    let url = detail
        .ti_items
        .iter()
        .find(|ti| ti.ti_file_flag.as_deref() == Some("thumbnail_1"))?
        .ti_storages
        .first()?;
    Some(url.replace("-ndr-private.", "-ndr."))
}

async fn fetch_raw_books(version: &DataVersion) -> Result<Vec<RawBook>, String> {
    let requests = version
        .url_list()
        .into_iter()
        .map(|url| async move { http::get_json::<Vec<RawBook>>(&url).await });

    let mut all_books = Vec::new();
    for result in join_all(requests).await {
        all_books.extend(result?);
    }
    Ok(all_books)
}

// 返回 id -> (total_uv, like_count)；统计失败不影响主流程，计为 0
pub async fn fetch_statistics(book_ids: &[String]) -> HashMap<String, (i64, i64)> {
    if book_ids.is_empty() {
        return HashMap::new();
    }

    let requests = book_ids
        .chunks(STATISTICS_BATCH_SIZE)
        .map(|chunk| async move {
            let url = format!("{STATISTICS_URL}{}", chunk.join(","));
            match http::get_json::<Vec<serde_json::Value>>(&url).await {
                Ok(stats) => stats,
                Err(e) => {
                    log::warn!("获取统计数据失败: {e}");
                    Vec::new()
                }
            }
        });

    join_all(requests)
        .await
        .into_iter()
        .flatten()
        .filter_map(|stat| {
            let id = stat.get("id")?.as_str()?.to_string();
            let uv = stat.get("total_uv").and_then(|v| v.as_i64()).unwrap_or(0);
            let likes = stat.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0);
            Some((id, (uv, likes)))
        })
        .collect()
}

// tag_path 形如 "id1/id2/.../idN"，判断是否包含 required 的连续子序列
fn path_matches(tag_path: &str, required: &[String]) -> bool {
    let parts: Vec<&str> = tag_path.split('/').collect();
    parts
        .windows(required.len())
        .any(|window| window.iter().zip(required).all(|(part, id)| part == id))
}

pub fn filter_books<'a>(books: &'a [RawBook], required: &[String]) -> Vec<&'a RawBook> {
    books
        .iter()
        .filter(|book| {
            book.tag_paths
                .iter()
                .any(|path| path_matches(path, required))
        })
        .collect()
}

pub fn to_textbooks(books: Vec<&RawBook>, stats: &HashMap<String, (i64, i64)>) -> Vec<Textbook> {
    books
        .into_iter()
        .map(|book| {
            let (total_uv, like_count) = stats.get(&book.id).copied().unwrap_or((0, 0));
            Textbook {
                id: book.id.clone(),
                cover_url: book
                    .custom_properties
                    .as_ref()
                    .map(select_cover_url)
                    .unwrap_or_default(),
                title: book.title.clone(),
                total_uv,
                like_count,
                download_url: format!("{DOWNLOAD_URL_PREFIX}{}{DOWNLOAD_URL_SUFFIX}", book.id),
            }
        })
        .collect()
}

// 目录字段仅作详情不可用时的封面兜底：preview 与 thumbnails 都可能滞后
// （preview 可能是课程包装层的旧转码，thumbnails 可能是旧版扫描图），
// 取转码时间戳较新的一个
fn select_cover_url(cp: &CustomProperties) -> String {
    let preview = first_preview_slide(cp);
    let thumbnail = cp
        .thumbnails
        .as_ref()
        .and_then(|thumbs| thumbs.first())
        .cloned();

    match (preview, thumbnail) {
        (Some(p), Some(t)) => {
            if transcode_timestamp(&t) > transcode_timestamp(&p) {
                t
            } else {
                p
            }
        }
        (Some(p), None) => p,
        (None, Some(t)) => t,
        (None, None) => String::new(),
    }
}

// 提取转码路径 ".t/zh-CN/{时间戳}/" 中的时间戳，非转码路径计为 0
fn transcode_timestamp(url: &str) -> u64 {
    url.split(".t/zh-CN/")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|ts| ts.parse().ok())
        .unwrap_or(0)
}

// preview 的键形如 "Slide1"/"Slide2"，取编号最小的即第一页封面
fn first_preview_slide(cp: &CustomProperties) -> Option<String> {
    cp.preview
        .as_ref()?
        .as_object()?
        .iter()
        .filter_map(|(key, value)| {
            let n: u32 = key
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()?;
            let url = value.as_str()?;
            (!url.is_empty()).then(|| (n, url.to_string()))
        })
        .min_by_key(|(n, _)| *n)
        .map(|(_, url)| url)
}
