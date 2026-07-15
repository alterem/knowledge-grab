use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Textbook {
    pub id: String,
    pub cover_url: String,
    pub title: String,
    pub total_uv: i64,
    pub like_count: i64,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawTag {
    pub tag_id: String,
    pub tag_name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterOptionsArgs {
    pub category_id: Option<String>,
    pub subject_id: Option<String>,
    pub version_id: Option<String>,
    pub grade_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DropdownOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagChild {
    pub tag_id: String,
    pub tag_name: String,
    pub hierarchies: Option<Vec<TagHierarchy>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagHierarchy {
    pub children: Option<Vec<TagChild>>,
}

impl TagHierarchy {
    pub fn with_children(children: Vec<TagChild>) -> Self {
        Self {
            children: Some(children),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagApiResponse {
    pub hierarchies: Vec<TagHierarchy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataVersion {
    pub module: String,
    pub module_version: u64,
    pub urls: String,
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
                        .into_values()
                        .map(|tag_child| TagHierarchy::with_children(vec![tag_child]))
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
