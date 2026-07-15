use crate::models::{DataVersion, DropdownOption, TagApiResponse, TagChild, Textbook};
use base64::{Engine, engine::general_purpose::STANDARD};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::command;

const DATA_VERSION_URL: &str =
    "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/resources/tch_material/version/data_version.json";
const TCH_MATERIAL_TAG_URL: &str =
    "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/tags/tch_material_tag.json";
const STATISTICS_URL_FORMAT: &str =
    "https://x-api.ykt.eduyun.cn/proxy/cloud/v1/res_stats/actions/query?res_ids={}";
// Textbook PDFs moved from the public bucket (assets_document, now 403) to the
// auth-required private bucket. The private host keeps a stable `pdf.pdf` alias,
// so we still only need the resource id — but requests must carry the x-nd-auth
// header (see downloader::create_request).
const DOWNLOAD_URL_FORMAT: &str =
    "https://r1-ndr-private.ykt.cbern.com.cn/edu_product/esp/assets/{}.pkg/pdf.pdf";

const SPECIAL_EDUCATION: &str = "特殊教育";
const HIGH_SCHOOL: &str = "高中";

// Statistics endpoint takes ids in the query string; batch to keep the URL length bounded.
const STATISTICS_BATCH_SIZE: usize = 50;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

type TagHierarchyMap = HashMap<String, TagChild>;

static TCH_MATERIAL_TAG_CACHE: Lazy<Mutex<Option<Arc<TagHierarchyMap>>>> =
    Lazy::new(|| Mutex::new(None));

struct HierarchyNavigator<'a> {
    tag_hierarchy: &'a HashMap<String, TagChild>,
}

impl<'a> HierarchyNavigator<'a> {
    fn new(tag_hierarchy: &'a HashMap<String, TagChild>) -> Self {
        Self { tag_hierarchy }
    }

    fn find_child_in_hierarchies<'b>(
        hierarchies: &'b Option<Vec<crate::models::TagHierarchy>>,
        target_id: &str,
    ) -> Option<&'b TagChild> {
        hierarchies
            .as_ref()?
            .iter()
            .flat_map(|h| h.children.as_ref().into_iter().flatten())
            .find(|ch| ch.tag_id == target_id)
    }

    fn get_category(&self, category_id: &str) -> Option<&'a TagChild> {
        self.tag_hierarchy.get(category_id)
    }

    fn get_subject(&self, category_id: &str, subject_id: &str) -> Option<&TagChild> {
        let category = self.get_category(category_id)?;
        Self::find_child_in_hierarchies(&category.hierarchies, subject_id)
    }

    fn get_version(
        &self,
        category_id: &str,
        subject_id: &str,
        version_id: &str,
    ) -> Option<&TagChild> {
        let subject = self.get_subject(category_id, subject_id)?;
        Self::find_child_in_hierarchies(&subject.hierarchies, version_id)
    }

    fn get_grade(
        &self,
        category_id: &str,
        subject_id: &str,
        version_id: &str,
        grade_id: &str,
    ) -> Option<&TagChild> {
        let version = self.get_version(category_id, subject_id, version_id)?;
        Self::find_child_in_hierarchies(&version.hierarchies, grade_id)
    }
}

#[derive(Debug)]
struct ValidationResult {
    is_special_education: bool,
    is_valid: bool,
}

fn validate_parameters(
    category_id: &str,
    subject_id: &str,
    version_id: &str,
    grade_id: &str,
    year_id: &Option<String>,
    tag_hierarchy: &HashMap<String, TagChild>,
) -> ValidationResult {
    let is_special_education = tag_hierarchy
        .get(category_id)
        .is_some_and(|cat| cat.tag_name == SPECIAL_EDUCATION);

    let is_high_school = tag_hierarchy
        .get(category_id)
        .is_some_and(|cat| cat.tag_name == HIGH_SCHOOL);

    let is_valid = !category_id.is_empty()
        && !subject_id.is_empty()
        && !version_id.is_empty()
        && (is_high_school || !grade_id.is_empty())
        && (!is_special_education || year_id.is_some());

    ValidationResult {
        is_special_education,
        is_valid,
    }
}

fn build_required_id_sequence(
    category_id: &str,
    subject_id: &str,
    version_id: &str,
    grade_id: &str,
    year_id: &Option<String>,
    validation: &ValidationResult,
) -> Vec<String> {
    let mut sequence = vec![
        category_id.to_string(),
        subject_id.to_string(),
        version_id.to_string(),
    ];

    if !grade_id.is_empty() || validation.is_special_education {
        sequence.push(grade_id.to_string());
    }

    if validation.is_special_education {
        if let Some(year_id_val) = year_id {
            sequence.push(year_id_val.clone());
        }
    }

    sequence
}

async fn fetch_statistics(book_ids: &[String]) -> HashMap<String, serde_json::Value> {
    if book_ids.is_empty() {
        return HashMap::new();
    }

    let requests = book_ids.chunks(STATISTICS_BATCH_SIZE).map(|chunk| async move {
        let res_ids = chunk.join(",");
        let statistics_url = STATISTICS_URL_FORMAT.replace("{}", &res_ids);

        println!("Fetching statistics from: {}", statistics_url);

        let response = match HTTP_CLIENT.get(&statistics_url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Failed to fetch statistics: {}", e);
                return Vec::new();
            }
        };

        if !response.status().is_success() {
            eprintln!(
                "Failed to fetch statistics: HTTP status {}",
                response.status()
            );
            return Vec::new();
        }

        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Failed to get statistics response body: {}", e);
                return Vec::new();
            }
        };

        match serde_json::from_str::<Vec<serde_json::Value>>(&body) {
            Ok(stats) => stats,
            Err(e) => {
                eprintln!(
                    "Failed to parse statistics JSON: {}. Response body: {}",
                    e, body
                );
                Vec::new()
            }
        }
    });

    let batches = futures_util::future::join_all(requests).await;

    let mut statistics_map = HashMap::new();
    for stat_obj in batches.into_iter().flatten() {
        if let Some(id) = stat_obj.get("id").and_then(|v| v.as_str()) {
            statistics_map.insert(id.to_string(), stat_obj);
        } else {
            eprintln!("Statistics object missing 'id' field: {:?}", stat_obj);
        }
    }

    statistics_map
}

fn convert_to_textbooks(
    raw_books: Vec<&crate::models::RawBook>,
    statistics: &HashMap<String, serde_json::Value>,
) -> Vec<Textbook> {
    raw_books
        .into_iter()
        .map(|raw_book| {
            let (total_uv, like_count) = statistics.get(&raw_book.id).map_or((0, 0), |s| {
                let total_uv = s.get("total_uv").and_then(|v| v.as_i64()).unwrap_or(0);
                let like_count = s.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0);
                (total_uv, like_count)
            });

            let download_url = DOWNLOAD_URL_FORMAT.replace("{}", &raw_book.id);
            let cover_url = raw_book
                .custom_properties
                .as_ref()
                .and_then(|cp| cp.thumbnails.as_ref())
                .and_then(|thumbs| thumbs.first())
                .cloned()
                .unwrap_or_default();

            Textbook {
                id: raw_book.id.clone(),
                cover_url,
                title: raw_book.title.clone(),
                total_uv,
                like_count,
                download_url,
            }
        })
        .collect()
}

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

    // The tag hierarchy (usually cached) and the data version are independent
    // network fetches, so run them concurrently and overlap their latency.
    let (tag_hierarchy_res, data_version_res) =
        tokio::join!(fetch_tch_material_tag(), fetch_and_parse_data_version());

    let tag_hierarchy = tag_hierarchy_res?;
    let validation = validate_parameters(
        &category_id,
        &subject_id,
        &version_id,
        &grade_id,
        &year_id,
        &tag_hierarchy,
    );

    if !validation.is_valid {
        println!("One or more required parameters are empty, returning empty textbook list.");
        return Ok(vec![]);
    }

    let required_id_sequence = build_required_id_sequence(
        &category_id,
        &subject_id,
        &version_id,
        &grade_id,
        &year_id,
        &validation,
    );

    println!(
        "Required ID sequence for filtering: {:?}",
        required_id_sequence
    );

    let data_version = data_version_res?;
    let raw_books = fetch_raw_books(data_version).await?;

    let filtered_raw_books: Vec<&crate::models::RawBook> = raw_books
        .iter()
        .filter(|book| {
            book.tag_paths.iter().any(|tag_path| {
                let path_parts: Vec<&str> = tag_path.split('/').collect();
                path_parts
                    .windows(required_id_sequence.len())
                    .any(|window| {
                        window
                            .iter()
                            .zip(&required_id_sequence)
                            .all(|(part, required_id)| part == required_id)
                    })
            })
        })
        .collect();

    let book_ids: Vec<String> = filtered_raw_books
        .iter()
        .map(|book| book.id.clone())
        .collect();

    let statistics = fetch_statistics(&book_ids).await;
    let textbooks = convert_to_textbooks(filtered_raw_books, &statistics);

    Ok(textbooks)
}

fn get_dropdown_options_from_hierarchies(
    hierarchies: &Option<Vec<crate::models::TagHierarchy>>,
) -> Vec<DropdownOption> {
    hierarchies
        .as_ref()
        .into_iter()
        .flatten()
        .flat_map(|h| h.children.as_ref().into_iter().flatten())
        .map(|child| DropdownOption {
            value: child.tag_id.clone(),
            label: child.tag_name.clone(),
        })
        .collect()
}

#[command]
pub async fn fetch_filter_options(
    args: crate::models::FilterOptionsArgs,
) -> Result<Vec<DropdownOption>, String> {
    println!(
        "Fetching filter options for category: {:?}, subject: {:?}, version: {:?}, grade: {:?}",
        args.category_id, args.subject_id, args.version_id, args.grade_id
    );

    let tag_hierarchy = fetch_tch_material_tag().await?;
    let navigator = HierarchyNavigator::new(&tag_hierarchy);

    let options = match args.category_id {
        Some(cat_id) => {
            if let Some(subject_id) = args.subject_id {
                if let Some(version_id) = args.version_id {
                    if let Some(grade_id) = args.grade_id {
                        navigator
                            .get_grade(&cat_id, &subject_id, &version_id, &grade_id)
                            .map(|grade| get_dropdown_options_from_hierarchies(&grade.hierarchies))
                            .unwrap_or_default()
                    } else if let Some(subject) = navigator.get_subject(&cat_id, &subject_id) {
                        if subject.tag_name == HIGH_SCHOOL {
                            println!("High school subject selected, no grades available.");
                            Vec::new()
                        } else {
                            navigator
                                .get_version(&cat_id, &subject_id, &version_id)
                                .map(|version| {
                                    get_dropdown_options_from_hierarchies(&version.hierarchies)
                                })
                                .unwrap_or_default()
                        }
                    } else {
                        Vec::new()
                    }
                } else if let Some(subject) = navigator.get_subject(&cat_id, &subject_id) {
                    get_dropdown_options_from_hierarchies(&subject.hierarchies)
                } else {
                    Vec::new()
                }
            } else if let Some(category) = navigator.get_category(&cat_id) {
                get_dropdown_options_from_hierarchies(&category.hierarchies)
            } else {
                Vec::new()
            }
        }
        None => Vec::new(),
    };

    Ok(options)
}

#[command]
pub async fn fetch_textbook_categories() -> Result<Vec<DropdownOption>, String> {
    println!("Fetching textbook categories");
    let tag_hierarchy = fetch_tch_material_tag().await?;

    let categories = tag_hierarchy
        .values()
        .map(|tag_child| DropdownOption {
            value: tag_child.tag_id.clone(),
            label: tag_child.tag_name.clone(),
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
    let response = HTTP_CLIENT
        .get(DATA_VERSION_URL)
        .send()
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

pub async fn fetch_tch_material_tag() -> Result<Arc<TagHierarchyMap>, String> {
    {
        let cache = TCH_MATERIAL_TAG_CACHE.lock().unwrap();
        if let Some(cached_data) = cache.as_ref() {
            println!("Using cached textbook material tags data");
            return Ok(Arc::clone(cached_data));
        }
    }

    println!(
        "Fetching and parsing textbook material tags from: {}",
        TCH_MATERIAL_TAG_URL
    );
    let response = HTTP_CLIENT
        .get(TCH_MATERIAL_TAG_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch textbook material tags: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch textbook material tags: HTTP status {}",
            response.status()
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to get response body as text: {}", e))?;
    let tag_api_response: TagApiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse textbook material tags JSON: {}", e))?;

    let hierarchies_to_parse = tag_api_response
        .hierarchies
        .into_iter()
        .next()
        .and_then(|h| h.children)
        .and_then(|children_vec| children_vec.into_iter().next())
        .and_then(|ch| ch.hierarchies)
        .unwrap_or_default();

    let parsed_hierarchy = Arc::new(crate::models::parse_hierarchies_recursive(Some(
        hierarchies_to_parse,
    )));

    {
        let mut cache = TCH_MATERIAL_TAG_CACHE.lock().unwrap();
        *cache = Some(Arc::clone(&parsed_hierarchy));
        println!("Cached textbook material tags data for future use");
    }

    Ok(parsed_hierarchy)
}

#[command]
pub async fn clear_tch_material_tag_cache() -> Result<(), String> {
    let mut cache = TCH_MATERIAL_TAG_CACHE.lock().unwrap();
    *cache = None;
    println!("Cleared textbook material tags cache");
    Ok(())
}

pub async fn fetch_and_parse_data_version() -> Result<DataVersion, String> {
    let json_value = fetch_data_version().await?;

    serde_json::from_value(json_value)
        .map_err(|e| format!("Failed to deserialize data version: {}", e))
}

#[command]
pub async fn fetch_image(url: String) -> Result<String, String> {
    println!("Fetching image from: {}", url);
    let response = HTTP_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch image: HTTP status {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to get image bytes: {}", e))?;
    let base64_image = Engine::encode(&STANDARD, &bytes);

    Ok(base64_image)
}

pub async fn fetch_raw_books(
    data_version: DataVersion,
) -> Result<Vec<crate::models::RawBook>, String> {
    println!("Fetching raw book data from URLs: {:?}", data_version.urls);
    let urls = data_version.get_urls();

    let mut tasks = vec![];

    for url in urls {
        tasks.push(tokio::spawn(async move {
            let response = HTTP_CLIENT
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch raw book data from {}: {}", url, e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Failed to fetch raw book data from {}: HTTP status {}",
                    url,
                    response.status()
                ));
            }
            let raw_books: Vec<crate::models::RawBook> = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse raw book data JSON from {}: {}", url, e))?;

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
