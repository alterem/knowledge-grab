use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Textbook {
    pub id: String,
    pub cover_url: String,
    pub title: String,
    pub total_uv: i64,
    pub like_count: i64,
    pub download_url: String,
}

#[derive(Deserialize, Debug)]
pub struct RawTag {
    pub tag_id: String,
    pub tag_name: String,
}

#[derive(Deserialize, Debug, Default)]
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

#[derive(Deserialize, Debug, Default)]
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

#[derive(Deserialize, Debug)]
pub struct FilterOptionsArgs {
    pub category_id: Option<String>,
    pub subject_id: Option<String>,
    pub version_id: Option<String>,
    pub grade_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DropdownOption {
    pub value: String,
    pub label: String,
}

#[derive(Serialize, Deserialize)]
pub struct FilterOptionsResponse {
    pub subjects: Vec<DropdownOption>,
    pub versions: Vec<DropdownOption>,
    pub grades: Vec<DropdownOption>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TagChild {
    pub tag_id: String,
    pub tag_name: String,
    pub hierarchies: Option<Vec<TagHierarchy>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TagHierarchy {
    pub children: Option<Vec<TagChild>>,
}

#[derive(Deserialize, Serialize)]
pub struct TagApiResponse {
    pub hierarchies: Vec<TagHierarchy>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct DataVersion {
    pub module: String,
    pub module_version: u64,
    pub urls: String,
}
pub fn parse_hierarchies_recursive(hierarchies: Option<Vec<TagHierarchy>>) -> HashMap<String, TagChild> {
    let mut parsed_map = HashMap::new();
    if let Some(hierarchies_vec) = hierarchies {
        for h in hierarchies_vec {
            if let Some(children_vec) = h.children {
                for ch in children_vec {
                    let tag_id = ch.tag_id.clone();
                    let parsed_children_map = parse_hierarchies_recursive(ch.hierarchies.clone());
                    let nested_hierarchies: Option<Vec<TagHierarchy>> = if parsed_children_map.is_empty() {
                        None
                    } else {
                        Some(parsed_children_map.into_iter().map(|(_, v)| TagHierarchy { children: Some(vec![v]) }).collect())
                    };

                    parsed_map.insert(tag_id, TagChild {
                        tag_id: ch.tag_id,
                        tag_name: ch.tag_name,
                        hierarchies: nested_hierarchies,
                    });
                }
            }
        }
    }
    parsed_map
}