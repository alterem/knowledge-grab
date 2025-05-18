use crate::models::{DropdownOption, Textbook, TagChild, TagApiResponse, DataVersion};
use tauri::command;
use std::collections::HashMap;
use reqwest;
use base64::{Engine, engine::general_purpose::STANDARD};
const DATA_VERSION_URL: &str = "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/resources/tch_material/version/data_version.json";
const TCH_MATERIAL_TAG_URL: &str = "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/tags/tch_material_tag.json";
const STATISTICS_URL_FORMAT: &str = "https://x-api.ykt.eduyun.cn/proxy/cloud/v1/res_stats/actions/query?res_ids={}";
const DOWNLOAD_URL_FORMAT: &str = "https://r2-ndr.ykt.cbern.com.cn/edu_product/esp/assets_document/{}.pkg/pdf.pdf";

#[command]
pub async fn fetch_textbooks(
    category_id: String,
    subject_id: String,
    version_id: String,
    grade_id: String,
    year_id: Option<String>,
) -> Result<Vec<Textbook>, String> {
    println!(
        "Fetching textbooks for category: {}, subject: {}, version: {}, grade: {}, year: {:?}",
        category_id, subject_id, version_id, grade_id, year_id
    );

    let tag_hierarchy = fetch_tch_material_tag().await?;
    let is_special_education = tag_hierarchy.get(&category_id).map_or(false, |cat| cat.tag_name == "特殊教育");
    // Allow empty grade_id for High School category
    let is_high_school = tag_hierarchy.get(&category_id).map_or(false, |cat| cat.tag_name == "高中");
    if category_id.is_empty() || subject_id.is_empty() || version_id.is_empty() || (!is_high_school && grade_id.is_empty()) || (is_special_education && year_id.is_none()) {
         println!("One or more required parameters are empty, returning empty textbook list.");
         return Ok(vec![]);
    }

    let data_version = fetch_and_parse_data_version().await?;
    let raw_books = fetch_raw_books(data_version).await?;

    let mut tag_names = Vec::new();
    if let Some(cat_child) = tag_hierarchy.get(&category_id) {
        tag_names.push(cat_child.tag_name.clone());
        if let Some(sub_child) = cat_child.hierarchies.as_ref().into_iter().flatten().flat_map(|h| h.children.as_ref().into_iter().flatten()).find(|ch| ch.tag_id == subject_id) {
            tag_names.push(sub_child.tag_name.clone());
            if let Some(ver_child) = sub_child.hierarchies.as_ref().into_iter().flatten().flat_map(|h| h.children.as_ref().into_iter().flatten()).find(|ch| ch.tag_id == version_id) {
                tag_names.push(ver_child.tag_name.clone());
                if is_special_education {
                    if let Some(grade_child) = ver_child.hierarchies.as_ref().into_iter().flatten().flat_map(|h| h.children.as_ref().into_iter().flatten()).find(|ch| ch.tag_id == grade_id) {
                         tag_names.push(grade_child.tag_name.clone());

                         if let Some(ref year_id_val) = year_id {
                             if let Some(year_child) = grade_child.hierarchies.as_ref().into_iter().flatten().flat_map(|h| h.children.as_ref().into_iter().flatten()).find(|ch| ch.tag_id == *year_id_val) {
                                 tag_names.push(year_child.tag_name.clone());
                             } else {
                                 println!("Year tag not found for ID: {}", year_id_val);
                             }
                         } else {
                             println!("Special Education category selected but year_id is missing.");
                         }
                    } else {
                        println!("Grade (学科 for SE) tag not found for ID: {}", grade_id);
                    }
                } else {
                    if let Some(grade_child) = ver_child.hierarchies.as_ref().into_iter().flatten().flat_map(|h| h.children.as_ref().into_iter().flatten()).find(|ch| ch.tag_id == grade_id) {
                        tag_names.push(grade_child.tag_name.clone());
                    } else {
                        println!("Grade tag not found for ID: {}", grade_id);
                    }
                }
            } else {
                println!("Version tag not found for ID: {}", version_id);
            }
        } else {
            println!("Subject tag not found for ID: {}", subject_id);
        }
    } else {
        println!("Category tag not found for ID: {}", category_id);
    }

    let mut required_id_sequence = vec![category_id.as_str(), subject_id.as_str(), version_id.as_str()];

    // Only include grade_id if it's not empty or if it's special education (where grade_id is required)
    if !grade_id.is_empty() || is_special_education {
        required_id_sequence.push(grade_id.as_str());
    }

    // Only include year_id if it's special education and year_id is present
    if is_special_education {
        if let Some(ref year_id_val) = year_id {
            required_id_sequence.push(year_id_val.as_str());
        }
    }

    println!("Required ID sequence for filtering: {:?}", required_id_sequence);

    let filtered_raw_books: Vec<&crate::models::RawBook> = raw_books
        .iter()
        .filter(|book| {
            book.tag_paths.iter().any(|tag_path| {
                let path_parts: Vec<&str> = tag_path.split('/').collect();
                path_parts.windows(required_id_sequence.len()).any(|window| {
                    window.iter().zip(&required_id_sequence).all(|(part, required_id)| part == required_id)
                })
            })
        })
        .collect();

    let book_ids: Vec<String> = filtered_raw_books.iter().map(|book| book.id.to_string()).collect();
    let res_ids = book_ids.join(",");

    let statistics_url = format!("{}", STATISTICS_URL_FORMAT.replace("{}", &res_ids));
    println!("Fetching statistics from: {}", statistics_url);

    let statistics: HashMap<String, serde_json::Value> = if res_ids.is_empty() {
        HashMap::new()
    } else {
        let response = reqwest::get(&statistics_url)
            .await
            .map_err(|e| format!("Failed to fetch statistics: {}", e))?;

        if !response.status().is_success() {
             eprintln!(
                 "Failed to fetch statistics: HTTP status {}",
                 response.status()
             );
             HashMap::new()
        } else {
            let body = response.text().await.map_err(|e| format!("Failed to get statistics response body as text: {}", e))?;
            let stats_list: Vec<serde_json::Value> = serde_json::from_str(&body)
                .map_err(|e| format!("Failed to parse statistics JSON: {}. Response body: {}", e, body))?;

            let mut statistics_map = HashMap::new();
            for stat_obj in stats_list {
                if let Some(id) = stat_obj.get("id").and_then(|v| v.as_str()) {
                    statistics_map.insert(id.to_string(), stat_obj);
                } else {
                    eprintln!("Statistics object missing 'id' field: {:?}", stat_obj);
                }
            }
            statistics_map
        }
    };

    let textbooks: Vec<Textbook> = filtered_raw_books
        .into_iter()
        .map(|raw_book| {
            let (total_uv, like_count) = statistics.get(&raw_book.id).map_or((0, 0), |s| {
                let total_uv = s.get("total_uv").and_then(|v| v.as_i64()).unwrap_or(0);
                let like_count = s.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0);
                (total_uv, like_count)
            });

            let download_url = format!("{}", DOWNLOAD_URL_FORMAT.replace("{}", &raw_book.id.to_string()));
            let cover_url = raw_book.custom_properties.as_ref()
                .and_then(|cp| cp.thumbnails.as_ref())
                .and_then(|thumbs| thumbs.get(0))
                .cloned()
                .unwrap_or_else(|| "".to_string());

            Textbook {
                id: raw_book.id.clone(),
                cover_url,
                title: raw_book.title.clone(),
                total_uv,
                like_count,
                download_url,
            }
        })
        .collect();

    Ok(textbooks)
}

#[command]
pub async fn fetch_filter_options(args: crate::models::FilterOptionsArgs) -> Result<Vec<DropdownOption>, String> {
    println!(
        "Fetching filter options for category: {:?}, subject: {:?}, version: {:?}, grade: {:?}",
        args.category_id, args.subject_id, args.version_id, args.grade_id
    );

    let tag_hierarchy = fetch_tch_material_tag().await?;

    let options = if let Some(cat_id) = args.category_id {
        if let Some(cat_child) = tag_hierarchy.get(&cat_id) {
            if let Some(sub_id) = args.subject_id {
                if let Some(sub_child) = cat_child.hierarchies.as_ref()
                    .into_iter()
                    .flatten()
                    .flat_map(|h| h.children.as_ref().into_iter().flatten())
                    .find(|ch| ch.tag_id == sub_id)
                {
                    if let Some(ver_id) = args.version_id {
                        if let Some(ver_child) = sub_child.hierarchies.as_ref()
                            .into_iter()
                            .flatten()
                            .flat_map(|h| h.children.as_ref().into_iter().flatten())
                            .find(|ch| ch.tag_id == ver_id)
                        {
                            if let Some(grade_id) = args.grade_id {
                                if let Some(grade_child) = ver_child.hierarchies.as_ref()
                                    .into_iter()
                                    .flatten()
                                    .flat_map(|h| h.children.as_ref().into_iter().flatten())
                                    .find(|ch| ch.tag_id == grade_id)
                                {
                                     grade_child.hierarchies.as_ref()
                                        .into_iter()
                                        .flatten()
                                        .flat_map(|h| h.children.as_ref().into_iter().flatten())
                                        .map(|year_child| DropdownOption {
                                            value: year_child.tag_id.clone(),
                                            label: year_child.tag_name.clone(),
                                        })
                                        .collect()
                                } else {
                                    Vec::new()
                                }

                            } else {
                                if sub_child.tag_name == "高中" {
                                    println!("High school subject selected, no grades available.");
                                    Vec::new()
                                } else {
                                    ver_child.hierarchies.as_ref()
                                        .into_iter()
                                        .flatten()
                                        .flat_map(|h| h.children.as_ref().into_iter().flatten())
                                        .map(|grade_child| DropdownOption {
                                            value: grade_child.tag_id.clone(),
                                            label: grade_child.tag_name.clone(),
                                        })
                                        .collect()
                                }
                            }
                        } else {
                            Vec::new()
                        }
                    } else {
                        sub_child.hierarchies.as_ref()
                            .into_iter()
                            .flatten()
                            .flat_map(|h| h.children.as_ref().into_iter().flatten())
                            .map(|version_child| DropdownOption {
                                value: version_child.tag_id.clone(),
                                label: version_child.tag_name.clone(),
                            })
                            .collect()
                    }
                } else {
                    Vec::new()
                }
            } else {
                cat_child.hierarchies.as_ref()
                    .into_iter()
                    .flatten()
                    .flat_map(|h| h.children.as_ref().into_iter().flatten())
                    .map(|subject_child| DropdownOption {
                        value: subject_child.tag_id.clone(),
                        label: subject_child.tag_name.clone(),
                    })
                    .collect()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(options)
}

#[command]
pub async fn fetch_textbook_categories() -> Result<Vec<DropdownOption>, String> {
    println!("Fetching textbook categories");
    let tag_hierarchy = fetch_tch_material_tag().await?;

    let categories = tag_hierarchy
        .into_iter()
        .map(|(_, tag_child)| DropdownOption {
            value: tag_child.tag_id,
            label: tag_child.tag_name,
        })
        .collect();

    Ok(categories)
}

impl DataVersion {
    pub fn get_urls(&self) -> Vec<String> {
        self.urls.split(',').map(|s| s.to_string()).collect()
    }
}

pub async fn fetch_data_version() -> Result<serde_json::Value, String> {
    println!("Fetching filter options from: {}", DATA_VERSION_URL);
    let response = reqwest::get(DATA_VERSION_URL)
        .await
        .map_err(|e| format!("Failed to fetch filter options: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch filter options: HTTP status {}",
            response.status()
        ));
    }

    let json_value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse filter options JSON: {}", e))?;

    Ok(json_value)
}

#[command]
pub async fn fetch_tch_material_tag() -> Result<HashMap<String, TagChild>, String> {
    println!("Fetching and parsing textbook material tags from: {}", TCH_MATERIAL_TAG_URL);
    let response = reqwest::get(TCH_MATERIAL_TAG_URL)
        .await
        .map_err(|e| format!("Failed to fetch textbook material tags: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch textbook material tags: HTTP status {}",
            response.status()
        ));
    }

    let body = response.text().await.map_err(|e| format!("Failed to get response body as text: {}", e))?;
    let tag_api_response: TagApiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse textbook material tags JSON: {}", e))?;
    let hierarchies_to_parse = tag_api_response
        .hierarchies
        .into_iter()
        .next()
        .and_then(|h| h.children)
        .and_then(|children_vec| children_vec.into_iter().next())
        .and_then(|ch| ch.hierarchies)
        .unwrap_or_else(Vec::new);
    let parsed_hierarchy = crate::models::parse_hierarchies_recursive(Some(hierarchies_to_parse));

    Ok(parsed_hierarchy)
}

pub async fn fetch_and_parse_data_version() -> Result<DataVersion, String> {
    let json_value = fetch_data_version().await?;

    serde_json::from_value(json_value)
        .map_err(|e| format!("Failed to deserialize data version: {}", e))
}

#[command]
pub async fn fetch_image(url: String) -> Result<String, String> {
    println!("Fetching image from: {}", url);
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch image: HTTP status {}",
            response.status()
        ));
    }

    let bytes = response.bytes().await.map_err(|e| format!("Failed to get image bytes: {}", e))?;
    let base64_image = Engine::encode(&STANDARD, &bytes);

    Ok(base64_image)
}

pub async fn fetch_raw_books(data_version: DataVersion) -> Result<Vec<crate::models::RawBook>, String> {
    println!("Fetching raw book data from URLs: {:?}", data_version.urls);
    let urls = data_version.get_urls();

    let client = reqwest::Client::new();
    let mut tasks = vec![];

    for url in urls {
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            let response = client.get(&url).send().await
                .map_err(|e| format!("Failed to fetch raw book data from {}: {}", url, e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Failed to fetch raw book data from {}: HTTP status {}",
                    url, response.status()
                ));
            }
            let raw_books: Vec<crate::models::RawBook> = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse raw book data JSON from {}: {}. Error details: {:?}", url, e, e))?;

            Ok(raw_books)
        }));
    }

    let mut all_raw_books: Vec<crate::models::RawBook> = vec![];
    for task in futures_util::future::join_all(tasks).await {
        match task {
            Ok(result) => all_raw_books.extend(result?),
            Err(e) => return Err(format!("Task failed: {}", e)),
        }
    }

    Ok(all_raw_books)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_data_version() {
        let result = fetch_data_version().await;
        assert!(result.is_ok(), "Failed to fetch data version: {:?}", result.err());

        let json_value = result.unwrap();
        println!("Fetched data version: {}", serde_json::to_string_pretty(&json_value).unwrap());
    }

    #[tokio::test]
    async fn test_fetch_and_parse_data_version() {
        let result = fetch_and_parse_data_version().await;
        assert!(result.is_ok(), "Failed to fetch and parse data version: {:?}", result.err());

        let data_version = result.unwrap();
        assert_eq!(data_version.module, "tch_material");
        assert!(data_version.module_version > 0);
        assert!(!data_version.urls.is_empty());
        let urls = data_version.get_urls();
        assert!(!urls.is_empty());
        for url in &urls {
            assert!(url.starts_with("https://"));
            assert!(url.contains("ykt.cbern.com.cn"));
        }

        println!("Module: {}", data_version.module);
        println!("Module Version: {}", data_version.module_version);
        println!("URLs: {:?}", urls);
    }
}

