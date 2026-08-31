use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct TextbookDownloadInfo {
    pub url: String,
    pub title: String,
    pub category_label: Option<String>,
    pub subject_label: Option<String>,
    pub version_label: Option<String>,
    pub grade_label: Option<String>,
    pub year_label: Option<String>,
    pub save_by_category: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Textbook {
    pub id: String,
    pub cover_url: String,
    pub title: String,
    pub total_uv: i64,
    pub like_count: i64,
    pub download_url: String,
}

// 一个课程 URL 解析后得到的资源（可能是视频，也可能是课件文档）。
// 前端据此展示列表并逐个下载。
#[derive(Debug, Clone, Serialize)]
pub struct CourseResource {
    pub id: String,
    pub title: String,
    // 文件后缀名（不含点），如 mp4 / pdf / pptx；视频统一标记为 mp4
    pub format: String,
    // m3u8 播放列表地址（视频）或文件直链（课件）
    pub download_url: String,
    // true 表示需要 m3u8 解密下载流程，false 表示直接流式下载
    pub is_video: bool,
    pub cover_url: String,
}

// 一个课程解析结果：课程标题 + 分类目录段（学段/学科/…，可为空）+ 其下的资源清单
#[derive(Debug, Clone, Serialize)]
pub struct CourseParseResult {
    pub title: String,
    pub category_path: Vec<String>,
    pub resources: Vec<CourseResource>,
}

// 目录页里的一节课。url 是该课时的详情页地址，可直接交给 parse_course_url 解析
#[derive(Debug, Clone, Serialize)]
pub struct CatalogLesson {
    pub id: String,
    pub title: String,
    // 所属章节，形如「第一单元 时、分、秒 / 秒的认识」，用于分组展示
    pub chapter: String,
    pub url: String,
    // 平台的资源类型（national_lesson / elite_lesson / knowledge_micro_lesson_package…）
    pub resource_type: String,
}

// 一册教材（同步课堂里的「整册课程」）及其全部课时
#[derive(Debug, Clone, Serialize)]
pub struct CatalogBook {
    pub id: String,
    pub title: String,
    pub category_path: Vec<String>,
    pub lessons: Vec<CatalogLesson>,
}

// 同步课堂目录页（列表页 / 整册教材页）的解析结果
#[derive(Debug, Clone, Serialize)]
pub struct CourseCatalog {
    pub title: String,
    pub category_path: Vec<String>,
    pub books: Vec<CatalogBook>,
}

// 一个链接的解析结果：要么是单个课程（含资源清单），要么是一份课时目录。
// 内部标签 kind 供前端区分（course / catalog）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CourseLink {
    Course(CourseParseResult),
    Catalog(CourseCatalog),
}

// 前端发起课程资源下载时回传的信息
#[derive(Debug, Clone, Deserialize)]
pub struct CourseDownloadInfo {
    pub download_url: String,
    pub title: String,
    pub format: String,
    pub is_video: bool,
    // 课程标题，用作保存子目录（同一课程的多个资源归在一起）
    pub course_title: Option<String>,
    // 「按分类保存」开关及解析出的分类目录段，勾选时插在课程子目录之前
    #[serde(default)]
    pub save_by_category: bool,
    #[serde(default)]
    pub category_path: Vec<String>,
}

// 只保留实际用到的字段，几千条书目反序列化后能省不少内存
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CustomProperties {
    #[serde(default)]
    pub preview: Option<Value>,
    #[serde(default)]
    pub thumbnails: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawBook {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tag_paths: Vec<String>,
    pub custom_properties: Option<CustomProperties>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterOptionsArgs {
    pub category_id: Option<String>,
    pub subject_id: Option<String>,
    pub version_id: Option<String>,
    pub grade_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DropdownOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagChild {
    pub tag_id: String,
    pub tag_name: String,
    pub hierarchies: Option<Vec<TagHierarchy>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagHierarchy {
    pub children: Option<Vec<TagChild>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagApiResponse {
    pub hierarchies: Vec<TagHierarchy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataVersion {
    pub module_version: u64,
    pub urls: String,
}

impl DataVersion {
    pub fn url_list(&self) -> Vec<String> {
        self.urls.split(',').map(str::to_string).collect()
    }
}
