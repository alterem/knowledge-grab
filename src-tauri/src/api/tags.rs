use crate::http;
use crate::models::{DropdownOption, TagApiResponse, TagChild, TagHierarchy};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

const TAG_URL: &str = "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/tags/tch_material_tag.json";

pub const SPECIAL_EDUCATION: &str = "特殊教育";
pub const HIGH_SCHOOL: &str = "高中";

// 顶层分类列表，保持接口原始顺序（前端菜单/下拉框按此顺序展示）
static TAG_TREE_CACHE: Lazy<Mutex<Option<Arc<Vec<TagChild>>>>> = Lazy::new(|| Mutex::new(None));

pub async fn fetch_tag_tree() -> Result<Arc<Vec<TagChild>>, String> {
    if let Some(cached) = TAG_TREE_CACHE.lock().unwrap().clone() {
        return Ok(cached);
    }

    log::info!("拉取教材标签数据: {TAG_URL}");
    let response: TagApiResponse = http::get_json(TAG_URL).await?;
    let categories = response
        .hierarchies
        .into_iter()
        .next()
        .and_then(|h| h.children)
        .and_then(|children| children.into_iter().next())
        .and_then(|child| child.hierarchies)
        .map(flatten_children)
        .unwrap_or_default();

    let tree = Arc::new(categories);
    *TAG_TREE_CACHE.lock().unwrap() = Some(Arc::clone(&tree));
    Ok(tree)
}

pub fn clear_cache() {
    *TAG_TREE_CACHE.lock().unwrap() = None;
}

fn flatten_children(hierarchies: Vec<TagHierarchy>) -> Vec<TagChild> {
    hierarchies
        .into_iter()
        .filter_map(|h| h.children)
        .flatten()
        .collect()
}

// 每一层的结构都是 hierarchies -> children
pub fn children_of(node: &TagChild) -> impl Iterator<Item = &TagChild> {
    node.hierarchies
        .iter()
        .flatten()
        .filter_map(|h| h.children.as_ref())
        .flatten()
}

pub fn find_category<'a>(tree: &'a [TagChild], id: &str) -> Option<&'a TagChild> {
    tree.iter().find(|c| c.tag_id == id)
}

// 按 分类/学科/版本/年级 的 tag_id 逐层向下查找
pub fn find_path<'a>(tree: &'a [TagChild], ids: &[&str]) -> Option<&'a TagChild> {
    let (first, rest) = ids.split_first()?;
    let mut node = find_category(tree, first)?;
    for id in rest {
        node = children_of(node).find(|c| c.tag_id == *id)?;
    }
    Some(node)
}

pub fn child_options(node: &TagChild) -> Vec<DropdownOption> {
    children_of(node)
        .map(|c| DropdownOption {
            value: c.tag_id.clone(),
            label: c.tag_name.clone(),
        })
        .collect()
}
