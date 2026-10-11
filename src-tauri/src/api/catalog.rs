use crate::api::courses;
use crate::http;
use crate::models::{CatalogBook, CatalogLesson, CourseCatalog, CourseLink, CourseParseResult};
use futures_util::stream::{self, StreamExt};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;
use url::Url;

// 同步课堂的目录数据都挂在静态文件桶上，与教材目录同一套路：
// 教材清单（按标签筛选）→ 章节树（决定课时顺序）→ 课时清单（拿到每节课的 id）。
// 课时详情沿用 courses.rs 已有的路由，故这里只负责把目录展开成详情页链接。
const FILE_HOST: &str = "https://s-file-1.ykt.cbern.com.cn/zxx";

// 目录展开与批量解析的并发上限。平台是静态桶，太高容易触发限流
const PARSE_CONCURRENCY: usize = 6;
// 一次目录解析最多展开的教材册数：筛选条件给得太宽（例如只选了「小学」）时
// 会命中上千册，直接请求会把平台和用户都拖垮，宁可提示用户选细一点
const MAX_BOOKS: usize = 8;

/// 同步课堂的两条线：学生自主学习（课程视频）与教师备课资源。
/// 两者的目录接口结构完全一致，只有路径前缀和章节树的接口版本不同。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Channel {
    NationalLesson,
    PrepareLesson,
}

impl Channel {
    fn slug(self) -> &'static str {
        match self {
            Channel::NationalLesson => "national_lesson",
            Channel::PrepareLesson => "prepare_lesson",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Channel::NationalLesson => "同步课堂",
            Channel::PrepareLesson => "教师备课资源",
        }
    }

    // 教材清单入口：学生侧带 data_version（可按版本缓存），备课侧只有 parts.json
    fn index_entry(self) -> String {
        match self {
            Channel::NationalLesson => format!(
                "{FILE_HOST}/ndrs/national_lesson/teachingmaterials/version/data_version.json"
            ),
            Channel::PrepareLesson => {
                format!("{FILE_HOST}/ndrs/prepare_lesson/teachingmaterials/parts.json")
            }
        }
    }

    fn detail_url(self, id: &str) -> String {
        format!(
            "{FILE_HOST}/ndrs/{}/teachingmaterials/details/{id}.json",
            self.slug()
        )
    }

    // 章节树：学生侧在 ndrv2，备课侧在 ndrs
    fn tree_url(self, id: &str) -> String {
        match self {
            Channel::NationalLesson => format!("{FILE_HOST}/ndrv2/national_lesson/trees/{id}.json"),
            Channel::PrepareLesson => format!("{FILE_HOST}/ndrs/prepare_lesson/trees/{id}.json"),
        }
    }

    fn lesson_parts_url(self, id: &str) -> String {
        format!(
            "{FILE_HOST}/ndrs/{}/teachingmaterials/{id}/resources/parts.json",
            self.slug()
        )
    }
}

// 目录链接指向的对象：一册具体教材，或一组筛选标签
enum CatalogTarget {
    Book {
        channel: Channel,
        id: String,
    },
    Tags {
        channel: Channel,
        tag_ids: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// 教材清单（按标签筛选用），按版本缓存
// ---------------------------------------------------------------------------

// 平台的目录数据里空值一律写成 null（而不是缺字段或空数组），凡是可能为空的
// 字段都得用 Option 接：漏一处，整份 JSON 就会反序列化失败
#[derive(Debug, Deserialize)]
struct IndexVersion {
    #[serde(default)]
    module_version: u64,
    #[serde(default)]
    urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTag {
    #[serde(default)]
    tag_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMaterial {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    tag_list: Option<Vec<RawTag>>,
}

impl RawMaterial {
    fn tag_ids(&self) -> HashSet<&str> {
        self.tag_list
            .iter()
            .flatten()
            .filter_map(|t| t.tag_id.as_deref())
            .collect()
    }
}

struct IndexCache {
    version: u64,
    materials: Arc<Vec<RawMaterial>>,
}

static INDEX_CACHE: Lazy<Mutex<HashMap<&'static str, IndexCache>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn clear_cache() {
    INDEX_CACHE.lock().await.clear();
}

// 教材清单有几 MB，按 module_version 缓存；备课侧没有版本号，进程内缓存一次
async fn material_index(channel: Channel) -> Result<Arc<Vec<RawMaterial>>, String> {
    let (version, mut part_urls) = match channel {
        Channel::NationalLesson => {
            let v: IndexVersion = http::get_json(&channel.index_entry())
                .await
                .map_err(|e| format!("获取教材清单版本失败: {e}"))?;
            (v.module_version, v.urls.unwrap_or_default())
        }
        Channel::PrepareLesson => (0, Vec::new()),
    };

    {
        let cache = INDEX_CACHE.lock().await;
        if let Some(cached) = cache.get(channel.slug()) {
            if cached.version == version {
                return Ok(Arc::clone(&cached.materials));
            }
        }
    }

    if part_urls.is_empty() {
        part_urls = http::get_json::<Vec<String>>(&channel.index_entry())
            .await
            .map_err(|e| format!("获取教材清单失败: {e}"))?;
    }

    log::info!(
        "拉取{}教材清单（{} 个分片）",
        channel.label(),
        part_urls.len()
    );
    let requests = part_urls
        .into_iter()
        .map(|url| async move { http::get_json::<Vec<RawMaterial>>(&url).await });

    let mut materials = Vec::new();
    for result in stream::iter(requests)
        .buffered(PARSE_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
    {
        materials.extend(result.map_err(|e| format!("获取教材清单失败: {e}"))?);
    }

    let materials = Arc::new(materials);
    INDEX_CACHE.lock().await.insert(
        channel.slug(),
        IndexCache {
            version,
            materials: Arc::clone(&materials),
        },
    );
    Ok(materials)
}

// ---------------------------------------------------------------------------
// 链接识别
// ---------------------------------------------------------------------------

// defaultTag 是用 / 连接的标签 id（地址栏里常被编码成 %2F）；
// 少数场景会拿到逗号分隔的写法，一并兼容
fn split_tag_ids(raw: &str) -> Vec<String> {
    raw.split(['/', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

// 逐级筛选（firstLevel/secondLevel/…）是 defaultTag 之外的另一种写法
fn level_tag_ids(url: &Url) -> Vec<String> {
    [
        "firstLevel",
        "secondLevel",
        "thirdLevel",
        "fourthLevel",
        "fifthLevel",
    ]
    .iter()
    .filter_map(|k| query_param(url, k))
    .filter(|v| !v.trim().is_empty())
    .collect()
}

// 目录/列表页 → 解析目标。单个课时详情页由调用方先行排除。
fn resolve_catalog(url: &Url) -> Option<CatalogTarget> {
    let path = url.path().trim_end_matches('/');
    // 平台上的路径大小写不统一（textBook / textbook 都出现过），匹配时统一小写；
    // 但 id 必须从原始路径上取，跟着小写化会拼出错误的接口地址
    let lower = path.to_lowercase();

    // 整册教材页：/syncClassroom/textBook/{教材id}
    if let Some(rest) = lower.strip_prefix("/syncclassroom/textbook/") {
        let id = path[path.len() - rest.len()..].trim();
        if !id.is_empty() {
            return Some(CatalogTarget::Book {
                channel: Channel::NationalLesson,
                id: id.to_string(),
            });
        }
    }

    // 列表页：/syncClassroom?defaultTag=… （备课资源为 /syncClassroom/prepare）
    let channel = match lower.as_str() {
        "/syncclassroom" | "/syncclassroom/auto" => Channel::NationalLesson,
        "/syncclassroom/prepare" => Channel::PrepareLesson,
        _ => return None,
    };

    let mut tag_ids = query_param(url, "defaultTag")
        .map(|raw| split_tag_ids(&raw))
        .unwrap_or_default();
    if tag_ids.is_empty() {
        tag_ids = level_tag_ids(url);
    }
    if tag_ids.is_empty() {
        return None;
    }
    Some(CatalogTarget::Tags { channel, tag_ids })
}

// ---------------------------------------------------------------------------
// 目录展开
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TreeNode {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    // 叶子节点的 child_nodes 是 null 而非 []
    #[serde(default)]
    child_nodes: Option<Vec<TreeNode>>,
}

#[derive(Debug, Deserialize)]
struct GlobalTitle {
    #[serde(rename = "zh-CN")]
    #[serde(default)]
    zh_cn: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawLesson {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    global_title: Option<GlobalTitle>,
    #[serde(default)]
    resource_type_code: Option<String>,
    // 形如 "单元id/课时id"，末段对应章节树上的节点
    #[serde(default)]
    chapter_paths: Option<Vec<String>>,
    #[serde(default)]
    status: Option<String>,
}

impl RawLesson {
    fn display_title(&self) -> String {
        self.title
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                self.global_title
                    .as_ref()
                    .and_then(|g| g.zh_cn.as_deref())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or("未命名课时")
            .trim()
            .to_string()
    }

    // 课时挂在章节树末段节点上；知识点微课等挂在另一棵树上，取不到就排到最后
    fn chapter_node_id(&self) -> &str {
        self.chapter_paths
            .iter()
            .flatten()
            .next()
            .and_then(|p| p.rsplit('/').next())
            .unwrap_or("")
    }
}

// 课时 id + 资源类型 → 平台上的详情页地址，交回 courses::resolve_route 解析。
// 返回 None 表示该类型没有可下载文件（如在线作答的习题/试卷）。
fn lesson_page_url(resource_type: &str, id: &str) -> Option<String> {
    let url = match resource_type {
        "national_lesson" => {
            format!("https://basic.smartedu.cn/syncClassroom/classActivity?activityId={id}")
        }
        "prepare_lesson" => {
            format!("https://basic.smartedu.cn/syncClassroom/prepare/detail?lessonId={id}")
        }
        "elite_lesson" => format!("https://basic.smartedu.cn/qualityCourse?courseId={id}"),
        // 习题、试卷是在线作答的互动内容，没有可下载的文件
        "examinationpapers" | "questions" | "homework" => return None,
        // 其余（知识点微课包、听力、演唱、课堂实录…）走通用资源详情页，
        // resourceType 即详情接口的资源桶名
        other if courses::is_safe_segment(other) => format!(
            "https://basic.smartedu.cn/syncClassroom/detail?resourceId={id}&resourceType={other}"
        ),
        _ => return None,
    };
    Some(url)
}

// 章节树拍平成 节点id → (先序序号, 从根到该节点的标题)
fn flatten_tree(nodes: &[TreeNode]) -> HashMap<String, (usize, Vec<String>)> {
    fn walk(
        nodes: &[TreeNode],
        parents: &[String],
        order: &mut usize,
        out: &mut HashMap<String, (usize, Vec<String>)>,
    ) {
        for node in nodes {
            let mut titles = parents.to_vec();
            let title = node.title.as_deref().unwrap_or_default().trim();
            if !title.is_empty() {
                titles.push(title.to_string());
            }
            *order += 1;
            if let Some(id) = node.id.as_deref().filter(|s| !s.is_empty()) {
                out.insert(id.to_string(), (*order, titles.clone()));
            }
            walk(
                node.child_nodes.as_deref().unwrap_or_default(),
                &titles,
                order,
                out,
            );
        }
    }

    let mut out = HashMap::new();
    walk(nodes, &[], &mut 0, &mut out);
    out
}

async fn fetch_lessons(channel: Channel, book_id: &str) -> Result<Vec<RawLesson>, String> {
    let part_urls: Vec<String> = http::get_json(&channel.lesson_parts_url(book_id))
        .await
        .map_err(|e| format!("获取课时清单失败: {e}"))?;

    let requests = part_urls
        .into_iter()
        .map(|url| async move { http::get_json::<Vec<RawLesson>>(&url).await });

    let mut lessons = Vec::new();
    for result in stream::iter(requests)
        .buffered(PARSE_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
    {
        lessons.extend(result.map_err(|e| format!("获取课时清单失败: {e}"))?);
    }
    Ok(lessons)
}

// 展开一册教材：详情（标题/分类）+ 章节树（顺序）+ 课时清单
async fn fetch_book(channel: Channel, id: &str) -> Result<CatalogBook, String> {
    let detail: Value = http::get_json(&channel.detail_url(id))
        .await
        .map_err(|e| format!("获取教材详情失败: {e}"))?;

    let title = detail
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            detail
                .get("global_title")
                .and_then(|g| g.get("zh-CN"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("未命名教材")
        .to_string();

    // 章节树缺失不致命（个别教材没有目录），课时退回按平台给的原顺序排
    let nodes: Vec<TreeNode> = http::get_json(&channel.tree_url(id))
        .await
        .unwrap_or_else(|e| {
            log::warn!("获取章节树失败，课时按原顺序排列: {e}");
            Vec::new()
        });
    let chapters = flatten_tree(&nodes);

    let raw_lessons = fetch_lessons(channel, id).await?;

    // 按 (章节序号, 原始序号) 排序：同一章节内保持平台给的先后
    let mut ordered: Vec<(usize, usize, CatalogLesson)> = Vec::new();
    let mut seen = HashSet::new();
    for (index, lesson) in raw_lessons.iter().enumerate() {
        let Some(lesson_id) = lesson.id.as_deref().filter(|s| !s.is_empty()) else {
            continue;
        };
        if lesson.status.as_deref().is_some_and(|s| s != "ONLINE") {
            continue;
        }
        let resource_type = lesson.resource_type_code.as_deref().unwrap_or_default();
        let Some(url) = lesson_page_url(resource_type, lesson_id) else {
            continue;
        };
        if !seen.insert(url.clone()) {
            continue;
        }

        let (order, titles) = chapters
            .get(lesson.chapter_node_id())
            .cloned()
            .unwrap_or((usize::MAX, Vec::new()));

        ordered.push((
            order,
            index,
            CatalogLesson {
                id: lesson_id.to_string(),
                title: lesson.display_title(),
                chapter: titles.join(" / "),
                url,
                resource_type: resource_type.to_string(),
            },
        ));
    }
    ordered.sort_by_key(|(order, index, _)| (*order, *index));

    Ok(CatalogBook {
        id: id.to_string(),
        title,
        category_path: courses::extract_category_path(&detail),
        lessons: ordered.into_iter().map(|(_, _, lesson)| lesson).collect(),
    })
}

// 按 defaultTag 的标签 id 找教材：标签是「与」关系，全部命中才算
async fn find_books(channel: Channel, tag_ids: &[String]) -> Result<Vec<RawMaterial>, String> {
    let materials = material_index(channel).await?;

    Ok(materials
        .iter()
        .filter(|m| {
            let own = m.tag_ids();
            tag_ids.iter().all(|id| own.contains(id.as_str()))
        })
        .cloned()
        .collect())
}

async fn build_catalog(target: CatalogTarget) -> Result<CourseCatalog, String> {
    let (channel, book_ids) = match target {
        CatalogTarget::Book { channel, id } => (channel, vec![id]),
        CatalogTarget::Tags { channel, tag_ids } => {
            let matched = find_books(channel, &tag_ids).await?;
            if matched.is_empty() {
                return Err(
                    "没有找到该筛选条件下的课程，请确认链接来自「课程教学 / 同步课堂」页面"
                        .to_string(),
                );
            }
            if matched.len() > MAX_BOOKS {
                return Err(format!(
                    "该链接匹配到 {} 册教材，范围太大。请在平台上把学段、年级、学科、版本、册次都选好后再复制链接",
                    matched.len()
                ));
            }
            log::info!(
                "目录命中 {} 册教材: {}",
                matched.len(),
                matched
                    .iter()
                    .filter_map(|m| m.title.as_deref())
                    .collect::<Vec<_>>()
                    .join("、")
            );
            (
                channel,
                matched.into_iter().filter_map(|m| m.id).collect::<Vec<_>>(),
            )
        }
    };

    let requests = book_ids
        .into_iter()
        .map(|id| async move { fetch_book(channel, &id).await });

    let books: Vec<CatalogBook> = stream::iter(requests)
        .buffered(PARSE_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter_map(|result| match result {
            Ok(book) => Some(book),
            Err(e) => {
                log::warn!("展开教材目录失败: {e}");
                None
            }
        })
        .filter(|book| !book.lessons.is_empty())
        .collect();

    if books.is_empty() {
        return Err("该目录下没有解析到可下载的课时".to_string());
    }

    // 单册时直接用教材名做标题，多册时概括数量
    let (title, category_path) = if books.len() == 1 {
        (books[0].title.clone(), books[0].category_path.clone())
    } else {
        (
            format!("{} · {} 册教材", channel.label(), books.len()),
            Vec::new(),
        )
    };

    Ok(CourseCatalog {
        title,
        category_path,
        books,
    })
}

// ---------------------------------------------------------------------------
// 命令
// ---------------------------------------------------------------------------

/// 解析任意课程链接：单个课时页返回资源清单，目录/列表页返回课时目录。
#[tauri::command]
pub async fn parse_course_link(url: String) -> Result<CourseLink, String> {
    let trimmed = url.trim();
    let parsed = Url::parse(trimmed).map_err(|e| format!("无效的链接: {e}"))?;

    // 单个课时/资源详情页优先，避免把 /syncClassroom/prepare/detail 之类误判成目录
    if courses::is_course_url(&parsed) {
        return courses::parse_course(trimmed).await.map(CourseLink::Course);
    }

    match resolve_catalog(&parsed) {
        Some(target) => build_catalog(target).await.map(CourseLink::Catalog),
        None => Err(
            "暂不支持该链接类型，请粘贴课程详情页，或「课程教学」里选好学段/学科/版本/年级/册次后的目录页地址"
                .to_string(),
        ),
    }
}

// 批量解析的进度，前端据此显示「已解析 x/y」
#[derive(Clone, serde::Serialize)]
struct ParseProgress {
    done: usize,
    total: usize,
    title: String,
}

/// 批量解析课时详情（整册下载用）。个别课时失败不影响整体，全部失败才报错。
#[tauri::command]
pub async fn parse_course_lessons(
    app_handle: tauri::AppHandle,
    urls: Vec<String>,
) -> Result<Vec<CourseParseResult>, String> {
    if urls.is_empty() {
        return Err("没有选择要解析的课时".to_string());
    }

    let total = urls.len();
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let requests = urls.into_iter().enumerate().map(|(index, url)| {
        let app_handle = app_handle.clone();
        let done = Arc::clone(&done);
        async move {
            let result = courses::parse_course(&url).await;
            let finished = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let title = match &result {
                Ok(course) => course.title.clone(),
                Err(e) => {
                    log::warn!("解析课时失败 {url}: {e}");
                    String::new()
                }
            };
            let _ = app_handle.emit(
                "course-parse-progress",
                ParseProgress {
                    done: finished,
                    total,
                    title,
                },
            );
            result.ok().map(|course| (index, course))
        }
    });

    let mut indexed: Vec<(usize, CourseParseResult)> = stream::iter(requests)
        .buffer_unordered(PARSE_CONCURRENCY)
        .filter_map(|item| async move { item })
        .collect::<Vec<_>>()
        .await;

    if indexed.is_empty() {
        return Err("所有课时都解析失败了，请稍后重试".to_string());
    }

    // 并发返回的顺序不定，按原始目录顺序还原
    indexed.sort_by_key(|(index, _)| *index);
    Ok(indexed.into_iter().map(|(_, course)| course).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_of(url: &str) -> Option<CatalogTarget> {
        resolve_catalog(&Url::parse(url).expect("测试 URL 应合法"))
    }

    #[test]
    fn recognizes_textbook_page() {
        let target = target_of(
            "https://basic.smartedu.cn/syncClassroom/textBook/92a82135-2748-4ec2-9bc4-e3fae0fb80f9",
        )
        .expect("整册教材页应被识别");
        match target {
            CatalogTarget::Book { channel, id } => {
                assert_eq!(channel, Channel::NationalLesson);
                assert_eq!(id, "92a82135-2748-4ec2-9bc4-e3fae0fb80f9");
            }
            _ => panic!("应识别为教材页"),
        }
    }

    // defaultTag 用 / 连接，从地址栏复制出来常被编码成 %2F
    #[test]
    fn recognizes_default_tag_list_page() {
        let target = target_of(
            "https://basic.smartedu.cn/syncClassroom?defaultTag=a1%2Fb2%2Fc3&reportHistory=true",
        )
        .expect("列表页应被识别");
        match target {
            CatalogTarget::Tags { channel, tag_ids } => {
                assert_eq!(channel, Channel::NationalLesson);
                assert_eq!(tag_ids, vec!["a1", "b2", "c3"]);
            }
            _ => panic!("应识别为标签筛选"),
        }
    }

    // id 里可能带大写字符，不能跟着路径一起被小写化
    #[test]
    fn keeps_book_id_case() {
        let target = target_of("https://basic.smartedu.cn/syncClassroom/textBook/A1b2-C3").unwrap();
        match target {
            CatalogTarget::Book { id, .. } => assert_eq!(id, "A1b2-C3"),
            _ => panic!("应识别为教材页"),
        }
    }

    #[test]
    fn recognizes_prepare_list_page() {
        let target = target_of("https://basic.smartedu.cn/syncClassroom/prepare?defaultTag=a1/b2")
            .expect("备课资源列表页应被识别");
        assert!(matches!(
            target,
            CatalogTarget::Tags {
                channel: Channel::PrepareLesson,
                ..
            }
        ));
    }

    // 单个课时页不能被当成目录（它们由 courses::resolve_route 处理）
    #[test]
    fn ignores_single_course_pages() {
        assert!(
            target_of("https://basic.smartedu.cn/syncClassroom/prepare/detail?lessonId=x")
                .is_none()
        );
        assert!(
            target_of("https://basic.smartedu.cn/syncClassroom/classActivity?activityId=x")
                .is_none()
        );
        // 没有任何筛选条件的裸列表页无从下手
        assert!(target_of("https://basic.smartedu.cn/syncClassroom").is_none());
    }

    #[test]
    fn maps_resource_types_to_detail_pages() {
        assert_eq!(
            lesson_page_url("national_lesson", "abc").unwrap(),
            "https://basic.smartedu.cn/syncClassroom/classActivity?activityId=abc"
        );
        assert_eq!(
            lesson_page_url("elite_lesson", "abc").unwrap(),
            "https://basic.smartedu.cn/qualityCourse?courseId=abc"
        );
        assert_eq!(
            lesson_page_url("knowledge_micro_lesson_package", "abc").unwrap(),
            "https://basic.smartedu.cn/syncClassroom/detail?resourceId=abc&resourceType=knowledge_micro_lesson_package"
        );
        // 在线作答的习题没有可下载文件
        assert!(lesson_page_url("examinationpapers", "abc").is_none());
        // 类型名会拼进 URL，非法取值直接丢弃
        assert!(lesson_page_url("../etc", "abc").is_none());
    }

    // 叶子节点的 child_nodes 是 null，反序列化必须容忍，否则整棵树丢失、
    // 课时会全部落到「无章节」并按原始顺序排
    #[test]
    fn flattens_tree_with_null_children() {
        let nodes: Vec<TreeNode> = serde_json::from_value(serde_json::json!([
            {
                "id": "u1",
                "title": "第一单元 时、分、秒",
                "child_nodes": [
                    {"id": "l1", "title": "秒的认识", "child_nodes": null},
                    {"id": "l2", "title": "解决问题", "child_nodes": null}
                ]
            },
            {"id": "u2", "title": "第二单元 测量", "child_nodes": null}
        ]))
        .unwrap();

        let map = flatten_tree(&nodes);
        assert_eq!(map["u1"].0, 1);
        assert_eq!(map["l1"].0, 2);
        assert_eq!(map["l2"].0, 3);
        assert_eq!(map["u2"].0, 4);
        assert_eq!(map["l1"].1, vec!["第一单元 时、分、秒", "秒的认识"]);
    }

    // 课时清单里的空值同样是 null，缺字段/空值都不能让整批解析失败
    #[test]
    fn lesson_tolerates_null_fields() {
        let lesson: RawLesson = serde_json::from_value(serde_json::json!({
            "id": "abc",
            "title": null,
            "global_title": {"zh-CN": "认识秒"},
            "resource_type_code": "national_lesson",
            "chapter_paths": null,
            "status": null
        }))
        .unwrap();
        assert_eq!(lesson.display_title(), "认识秒");
        assert_eq!(lesson.chapter_node_id(), "");
    }

    #[test]
    fn lesson_takes_last_segment_of_chapter_path() {
        let lesson: RawLesson = serde_json::from_value(serde_json::json!({
            "id": "abc",
            "chapter_paths": ["unit-1/lesson-2"]
        }))
        .unwrap();
        assert_eq!(lesson.chapter_node_id(), "lesson-2");
    }
}

// 联网校验目录接口是否还在（平台改版时最先失效的一环），默认不跑：
// cargo test live_ -- --ignored --nocapture
#[cfg(test)]
mod live_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn live_textbook_catalog() {
        let target = resolve_catalog(
            &Url::parse(
                "https://basic.smartedu.cn/syncClassroom/textBook/92a82135-2748-4ec2-9bc4-e3fae0fb80f9",
            )
            .unwrap(),
        )
        .unwrap();
        let catalog = build_catalog(target).await.unwrap();
        println!("catalog: {} {:?}", catalog.title, catalog.category_path);
        let book = &catalog.books[0];
        println!("lessons: {}", book.lessons.len());
        for lesson in book.lessons.iter().take(8) {
            println!(
                "  [{}] {} | {} -> {}",
                lesson.resource_type, lesson.chapter, lesson.title, lesson.url
            );
        }
        let course = courses::parse_course(&book.lessons[0].url).await.unwrap();
        println!(
            "first lesson resources: {:?}",
            course
                .resources
                .iter()
                .map(|r| (&r.title, &r.format, r.is_video))
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_prepare_channel_catalog() {
        // 备课侧的章节树在 ndrs、课时类型为 prepare_lesson，路径与学生侧不同，单独验一下
        let url = "https://basic.smartedu.cn/syncClassroom/prepare?defaultTag=e7bbb2de-0590-11ed-9c79-92fc3b3249d5%2Fe7bbd372-0590-11ed-9c79-92fc3b3249d5%2Fe7bbcf80-0590-11ed-9c79-92fc3b3249d5%2Fff8080814371757b01437c363a187b0a%2Fff8080814371757b014390f883db0453";
        let catalog = build_catalog(resolve_catalog(&Url::parse(url).unwrap()).unwrap())
            .await
            .unwrap();
        println!(
            "prepare catalog: {} books={}",
            catalog.title,
            catalog.books.len()
        );
        let book = &catalog.books[0];
        for lesson in book.lessons.iter().take(5) {
            println!(
                "  [{}] {} | {} -> {}",
                lesson.resource_type, lesson.chapter, lesson.title, lesson.url
            );
        }
        let course = courses::parse_course(&book.lessons[0].url).await.unwrap();
        println!(
            "first lesson resources: {:?}",
            course
                .resources
                .iter()
                .map(|r| (&r.title, &r.format, r.is_video))
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_default_tag_catalog() {
        // 小学 / 三年级 / 数学 / 人教版 / 上册
        let url = "https://basic.smartedu.cn/syncClassroom?defaultTag=e7bbb2de-0590-11ed-9c79-92fc3b3249d5%2Fe7bbd372-0590-11ed-9c79-92fc3b3249d5%2Fe7bbcf80-0590-11ed-9c79-92fc3b3249d5%2Fff8080814371757b01437c363a187b0a%2Fff8080814371757b014390f883db0453";
        let catalog = build_catalog(resolve_catalog(&Url::parse(url).unwrap()).unwrap())
            .await
            .unwrap();
        println!("catalog: {} books={}", catalog.title, catalog.books.len());
        for book in &catalog.books {
            println!(
                "  {} ({} 课时) {:?}",
                book.title,
                book.lessons.len(),
                book.category_path
            );
            for lesson in book.lessons.iter().take(3) {
                println!("    {} | {}", lesson.chapter, lesson.title);
            }
        }
    }
}
