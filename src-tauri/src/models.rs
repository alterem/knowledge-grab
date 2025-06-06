use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl TextbookDownloadInfo {
    pub fn new(url: String, title: String) -> Self {
        Self {
            url,
            title,
            category_label: None,
            subject_label: None,
            version_label: None,
            grade_label: None,
            year_label: None,
            save_by_category: false,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.url.is_empty() {
            return Err("下载URL不能为空".to_string());
        }

        if self.title.is_empty() {
            return Err("教科书标题不能为空".to_string());
        }

        Url::parse(&self.url).map_err(|_| "无效的URL格式".to_string())?;

        Ok(())
    }

    pub fn get_path_components(&self) -> Vec<String> {
        if !self.save_by_category {
            return vec![];
        }

        [
            &self.category_label,
            &self.subject_label,
            &self.version_label,
            &self.grade_label,
            &self.year_label,
        ]
        .iter()
        .filter_map(|label| label.as_ref())
        .filter(|label| !label.is_empty())
        .map(|label| label.clone())
        .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Textbook {
    pub id: String,
    pub cover_url: String,
    pub title: String,
    pub total_uv: i64,
    pub like_count: i64,
    pub download_url: String,
}

impl Textbook {
    pub fn new(id: String, title: String, download_url: String) -> Self {
        Self {
            id,
            cover_url: String::new(),
            title,
            total_uv: 0,
            like_count: 0,
            download_url,
        }
    }

    pub fn is_popular(&self) -> bool {
        self.total_uv > 1000 || self.like_count > 50
    }

    pub fn popularity_score(&self) -> u8 {
        let uv_score = (self.total_uv.min(10000) as f64 / 10000.0 * 70.0) as u8;
        let like_score = (self.like_count.min(500) as f64 / 500.0 * 30.0) as u8;
        (uv_score + like_score).min(100)
    }
}

impl fmt::Display for Textbook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (ID: {}, 浏览量: {}, 点赞: {})",
            self.title, self.id, self.total_uv, self.like_count
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawTag {
    pub tag_id: String,
    pub tag_name: String,
}

impl RawTag {
    pub fn new(tag_id: String, tag_name: String) -> Self {
        Self { tag_id, tag_name }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CustomProperties {
    #[serde(default)]
    pub size: Option<Value>,
    #[serde(default)]
    pub preview: Option<Value>,
    #[serde(default)]
    pub thumbnails: Option<Vec<String>>,
    #[serde(default)]
    pub format: Option<Value>,
}

impl CustomProperties {
    pub fn first_thumbnail(&self) -> Option<&String> {
        self.thumbnails.as_ref()?.first()
    }

    pub fn get_file_size(&self) -> Option<u64> {
        self.size.as_ref()?.as_u64()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RawBook {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tag_list: Vec<RawTag>,
    #[serde(default)]
    pub tag_paths: Vec<String>,
    pub custom_properties: Option<CustomProperties>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
}

impl RawBook {
    pub fn is_valid(&self) -> bool {
        !self.id.is_empty() && !self.title.is_empty()
    }

    pub fn primary_tag_path(&self) -> Option<&String> {
        self.tag_paths.first()
    }

    pub fn to_textbook(&self, download_url: String) -> Textbook {
        let cover_url = self
            .custom_properties
            .as_ref()
            .and_then(|cp| cp.first_thumbnail())
            .cloned()
            .unwrap_or_default();

        Textbook {
            id: self.id.clone(),
            cover_url,
            title: self.title.clone(),
            total_uv: 0,
            like_count: 0,
            download_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterOptionsArgs {
    pub category_id: Option<String>,
    pub subject_id: Option<String>,
    pub version_id: Option<String>,
    pub grade_id: Option<String>,
}

impl FilterOptionsArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_category(mut self, category_id: String) -> Self {
        self.category_id = Some(category_id);
        self
    }

    pub fn with_subject(mut self, subject_id: String) -> Self {
        self.subject_id = Some(subject_id);
        self
    }

    pub fn with_version(mut self, version_id: String) -> Self {
        self.version_id = Some(version_id);
        self
    }

    pub fn with_grade(mut self, grade_id: String) -> Self {
        self.grade_id = Some(grade_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DropdownOption {
    pub value: String,
    pub label: String,
}

impl DropdownOption {
    pub fn new(value: String, label: String) -> Self {
        Self { value, label }
    }
}

impl fmt::Display for DropdownOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.label, self.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOptionsResponse {
    pub subjects: Vec<DropdownOption>,
    pub versions: Vec<DropdownOption>,
    pub grades: Vec<DropdownOption>,
}

impl FilterOptionsResponse {
    pub fn empty() -> Self {
        Self {
            subjects: Vec::new(),
            versions: Vec::new(),
            grades: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagChild {
    pub tag_id: String,
    pub tag_name: String,
    pub hierarchies: Option<Vec<TagHierarchy>>,
}

impl TagChild {
    pub fn new(tag_id: String, tag_name: String) -> Self {
        Self {
            tag_id,
            tag_name,
            hierarchies: None,
        }
    }

    pub fn has_children(&self) -> bool {
        self.hierarchies.as_ref().map_or(false, |h| !h.is_empty())
    }

    pub fn children_count(&self) -> usize {
        self.hierarchies.as_ref().map_or(0, |hierarchies| {
            hierarchies
                .iter()
                .map(|h| h.children.as_ref().map_or(0, |c| c.len()))
                .sum()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagHierarchy {
    pub children: Option<Vec<TagChild>>,
}

impl TagHierarchy {
    pub fn new() -> Self {
        Self { children: None }
    }

    pub fn with_children(children: Vec<TagChild>) -> Self {
        Self {
            children: Some(children),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.children.as_ref().map_or(true, |c| c.is_empty())
    }
}

impl Default for TagHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagApiResponse {
    pub hierarchies: Vec<TagHierarchy>,
}

impl TagApiResponse {
    pub fn empty() -> Self {
        Self {
            hierarchies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataVersion {
    pub module: String,
    pub module_version: u64,
    pub urls: String,
}

impl DataVersion {
    pub fn new(module: String, module_version: u64, urls: String) -> Self {
        Self {
            module,
            module_version,
            urls,
        }
    }

    pub fn is_newer_than(&self, other: &DataVersion) -> bool {
        self.module == other.module && self.module_version > other.module_version
    }
}

impl fmt::Display for DataVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{}", self.module, self.module_version)
    }
}
pub fn parse_hierarchies_recursive(
    hierarchies: Option<Vec<TagHierarchy>>,
) -> HashMap<String, TagChild> {
    let mut parsed_map = HashMap::new();

    let Some(hierarchies_vec) = hierarchies else {
        return parsed_map;
    };

    for hierarchy in hierarchies_vec {
        let Some(children_vec) = hierarchy.children else {
            continue;
        };

        for child in children_vec {
            let tag_id = child.tag_id.clone();

            let parsed_children_map = parse_hierarchies_recursive(child.hierarchies.clone());

            let nested_hierarchies = if parsed_children_map.is_empty() {
                None
            } else {
                Some(
                    parsed_children_map
                        .into_iter()
                        .map(|(_, tag_child)| TagHierarchy::with_children(vec![tag_child]))
                        .collect(),
                )
            };

            parsed_map.insert(
                tag_id,
                TagChild {
                    tag_id: child.tag_id,
                    tag_name: child.tag_name,
                    hierarchies: nested_hierarchies,
                },
            );
        }
    }

    parsed_map
}
