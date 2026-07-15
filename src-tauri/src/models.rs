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
