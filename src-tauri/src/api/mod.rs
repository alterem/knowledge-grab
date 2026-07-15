pub mod books;
pub mod tags;

use crate::http;
use crate::models::{DropdownOption, FilterOptionsArgs, Textbook};
use base64::{Engine, engine::general_purpose::STANDARD};
use tags::{HIGH_SCHOOL, SPECIAL_EDUCATION};
use tauri::command;

#[command]
pub async fn fetch_textbooks(
    category_id: String,
    subject_id: String,
    version_id: String,
    grade_id: String,
    year_id: Option<String>,
) -> Result<Vec<Textbook>, String> {
    let tree = tags::fetch_tag_tree().await?;

    let category = tags::find_category(&tree, &category_id);
    let is_special = category.is_some_and(|c| c.tag_name == SPECIAL_EDUCATION);
    let is_high_school = category.is_some_and(|c| c.tag_name == HIGH_SCHOOL);

    // 高中没有年级层级；特殊教育多一层「年份」
    let is_valid = !category_id.is_empty()
        && !subject_id.is_empty()
        && !version_id.is_empty()
        && (is_high_school || !grade_id.is_empty())
        && (!is_special || year_id.is_some());
    if !is_valid {
        return Ok(vec![]);
    }

    let mut required = vec![category_id, subject_id, version_id];
    if !grade_id.is_empty() || is_special {
        required.push(grade_id);
    }
    if is_special {
        if let Some(year) = year_id {
            required.push(year);
        }
    }

    let raw_books = books::get_raw_books().await?;
    let filtered = books::filter_books(&raw_books, &required);
    let ids: Vec<String> = filtered.iter().map(|book| book.id.clone()).collect();
    let stats = books::fetch_statistics(&ids).await;

    Ok(books::to_textbooks(filtered, &stats))
}

#[command]
pub async fn fetch_filter_options(args: FilterOptionsArgs) -> Result<Vec<DropdownOption>, String> {
    let tree = tags::fetch_tag_tree().await?;

    let Some(cat) = args.category_id.as_deref() else {
        return Ok(vec![]);
    };

    let options = match (
        args.subject_id.as_deref(),
        args.version_id.as_deref(),
        args.grade_id.as_deref(),
    ) {
        (None, ..) => tags::find_path(&tree, &[cat]).map(tags::child_options),
        (Some(subj), None, _) => tags::find_path(&tree, &[cat, subj]).map(tags::child_options),
        (Some(subj), Some(ver), None) => match tags::find_path(&tree, &[cat, subj]) {
            // 高中层级下没有年级
            Some(subject) if subject.tag_name == HIGH_SCHOOL => None,
            Some(_) => tags::find_path(&tree, &[cat, subj, ver]).map(tags::child_options),
            None => None,
        },
        (Some(subj), Some(ver), Some(grade)) => {
            tags::find_path(&tree, &[cat, subj, ver, grade]).map(tags::child_options)
        }
    };

    Ok(options.unwrap_or_default())
}

#[command]
pub async fn fetch_textbook_categories() -> Result<Vec<DropdownOption>, String> {
    let tree = tags::fetch_tag_tree().await?;
    Ok(tree
        .iter()
        .map(|c| DropdownOption {
            value: c.tag_id.clone(),
            label: c.tag_name.clone(),
        })
        .collect())
}

#[command]
pub async fn clear_tch_material_tag_cache() -> Result<(), String> {
    tags::clear_cache();
    books::clear_cache().await;
    log::info!("已清理标签与书目缓存");
    Ok(())
}

// 封面直连有防盗链，由后端代理拉取并转 base64 给前端。
// 优先用详情解析出的、与源 PDF 同批的转码首页图；详情不可用时退回目录字段选出的地址
#[command]
pub async fn fetch_cover(book_id: String, fallback_url: String) -> Result<String, String> {
    if let Some(url) = books::resolve_cover_url(&book_id).await {
        match http::get_bytes(&url).await {
            Ok(bytes) => return Ok(STANDARD.encode(&bytes)),
            Err(e) => log::warn!("详情封面获取失败，使用目录封面: {e}"),
        }
    }

    if fallback_url.is_empty() {
        return Err("无可用封面".to_string());
    }
    let bytes = http::get_bytes(&fallback_url).await?;
    Ok(STANDARD.encode(&bytes))
}
