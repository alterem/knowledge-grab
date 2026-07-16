use crate::http;
use crate::models::{CourseParseResult, CourseResource};
use serde_json::Value;
use url::Url;

// 平台各类课程页 URL → 详情接口的映射。绝大多数详情 JSON 的形态一致：
// 要么顶层直接带 ti_items（单个资源），要么带 relations（命名的资源数组）。
// 参考 52beijixing/smartedu-download 的分发表，用当前线上接口校准。

// 视频文件格式，命中则走 m3u8 解密下载流程
const VIDEO_FORMATS: &[&str] = &["mp4", "m3u8", "avi", "flv", "mov"];

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

// 详情接口地址 + 需要从 relations 里提取的键（空则表示资源在顶层 ti_items）
struct Route {
    detail_url: String,
    relation_keys: Vec<&'static str>,
}

// 依据课程页 URL 推断详情接口。返回 None 表示暂不支持的链接类型。
fn resolve_route(url: &Url) -> Option<Route> {
    let path = url.path();
    let host = url.host_str().unwrap_or("");

    // 同步课堂 - 课程视频（书课包）
    if path.starts_with("/syncClassroom/classActivity") {
        let id = query_param(url, "activityId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-2.ykt.cbern.com.cn/zxx/ndrv2/national_lesson/resources/details/{id}.json"
            ),
            relation_keys: vec!["national_course_resource"],
        });
    }

    // 同步课堂 - 一师一优课（含教学设计、课堂实录视频、教学素材）
    if path.starts_with("/syncClassroom/prepare/detail") {
        if let Some(id) = query_param(url, "lessonId") {
            return Some(Route {
                detail_url: format!(
                    "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/prepare_lesson/resources/details/{id}.json"
                ),
                relation_keys: vec![
                    "lesson_plan_design",
                    "classroom_record",
                    "teaching_assets",
                ],
            });
        }
        // 备课资源（单个课件）
        let id = query_param(url, "resourceId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-2.ykt.cbern.com.cn/zxx/ndrv2/prepare_sub_type/resources/details/{id}.json"
            ),
            relation_keys: vec![],
        });
    }

    // 同步课堂 - 实验课
    if path.starts_with("/syncClassroom/experimentLesson") {
        let id = query_param(url, "courseId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/experiment/resources/details/{id}.json"
            ),
            relation_keys: vec!["lesson_1", "experiment_video"],
        });
    }

    // 同步课堂 - 基础作业（文档）
    if path.starts_with("/syncClassroom/basicWork/detail") {
        let id = query_param(url, "contentId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/special_edu/resources/details/{id}.json"
            ),
            relation_keys: vec![],
        });
    }

    // 学科精品课
    if path.starts_with("/qualityCourse") {
        let id = query_param(url, "courseId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/resources/{id}.json"
            ),
            relation_keys: vec!["course_resource"],
        });
    }

    // 基础教育精品课（jpk 子站，年度评优课）
    if path.starts_with("/yearQualityCourse") || host.starts_with("jpk.") {
        let id = query_param(url, "courseId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/competitive/elite_lesson/resources/{id}.json"
            ),
            relation_keys: vec!["course_resource"],
        });
    }

    // 德育/思政视频
    if path.starts_with("/sedu/detail") {
        let id = query_param(url, "contentId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/special_edu/resources/details/{id}.json"
            ),
            relation_keys: vec![],
        });
    }

    // 智慧教育视频
    if path.starts_with("/wisdom/detail") {
        let id = query_param(url, "contentId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/ldjy/ndrs/special_edu/resources/details/{id}.json"
            ),
            relation_keys: vec![],
        });
    }

    None
}

// 从一个资源对象（顶层详情或 relations 里的元素）里解析出可下载资源。
// 无可用下载地址时返回 None（例如需 doc-center 二次鉴权的课件，留待第二步支持）。
fn extract_resource(obj: &Value, fallback_title: &str) -> Option<CourseResource> {
    let cp = obj.get("custom_properties");
    let format = cp
        .and_then(|c| c.get("format"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();

    // 标题优先取 global_title.zh-CN，退回 title
    let title = obj
        .get("global_title")
        .and_then(|g| g.get("zh-CN"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| obj.get("title").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_title)
        .to_string();

    let cover_url = cp
        .and_then(|c| c.get("preview"))
        .and_then(|p| p.get("frame1"))
        .and_then(Value::as_str)
        .or_else(|| {
            cp.and_then(|c| c.get("thumbnails"))
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(Value::as_str)
        })
        .unwrap_or("")
        .to_string();

    let ti_items = obj.get("ti_items").and_then(Value::as_array)?;
    let has_m3u8 = ti_items
        .iter()
        .any(|it| it.get("ti_format").and_then(Value::as_str) == Some("m3u8"));
    let is_video = has_m3u8 || VIDEO_FORMATS.contains(&format.as_str());

    let download_url = if is_video {
        pick_video_url(ti_items)?
    } else {
        pick_file_url(ti_items)?
    };

    let out_format = if is_video && (format.is_empty() || format == "m3u8") {
        "mp4".to_string()
    } else if format.is_empty() {
        // 从 URL 后缀兜底推断
        url_extension(&download_url).unwrap_or_else(|| "bin".to_string())
    } else {
        format
    };

    Some(CourseResource {
        id: obj
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        title,
        format: out_format,
        download_url,
        is_video,
        cover_url,
    })
}

// 从 ti_items 里挑 m3u8 播放列表，优先 720p，其次任意 m3u8。
fn pick_video_url(ti_items: &[Value]) -> Option<String> {
    let m3u8_items: Vec<&Value> = ti_items
        .iter()
        .filter(|it| it.get("ti_format").and_then(Value::as_str) == Some("m3u8"))
        .collect();

    let chosen = m3u8_items
        .iter()
        .find(|it| it.get("ti_file_flag").and_then(Value::as_str) == Some("href-720p-m3u8"))
        .or_else(|| m3u8_items.first())?;

    first_storage(chosen)
}

// 从 ti_items 里挑课件文档的直链：优先 ti_file_flag=="href"，其次 "source"
// （基础作业等 special_edu 文档用 source 标源 PDF）。兜底跳过 folder 目录项，
// 其地址不可直接下载（403 会被误报成 token 失效）。
fn pick_file_url(ti_items: &[Value]) -> Option<String> {
    let flag_is = |it: &&Value, v: &str| {
        it.get("ti_file_flag").and_then(Value::as_str) == Some(v)
    };
    let chosen = ti_items
        .iter()
        .find(|it| flag_is(it, "href"))
        .or_else(|| ti_items.iter().find(|it| flag_is(it, "source")))
        .or_else(|| {
            ti_items
                .iter()
                .find(|it| it.get("ti_format").and_then(Value::as_str) != Some("folder"))
        })?;

    first_storage(chosen)
}

// ti_storages 是含 r1/r2/r3 镜像的完整 URL 数组，取第一个。
fn first_storage(item: &Value) -> Option<String> {
    item.get("ti_storages")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .filter(|s| s.starts_with("http"))
        .map(str::to_string)
}

fn url_extension(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let last = path.rsplit('/').next()?;
    let ext = last.rsplit_once('.')?.1;
    if ext.is_empty() || ext.len() > 5 {
        None
    } else {
        Some(ext.to_lowercase())
    }
}

// 已知标签维度 → 目录层级顺序：学段/学科/版本/年级/册次，与教材「按分类保存」的
// 层级习惯一致。其余维度（如 bklx=课程包 这类资源类型标签）不入目录。
const TAG_DIMENSION_ORDER: &[&str] = &["zxxxd", "zxxxk", "zxxbb", "zxxnj", "zxxcc"];

// 从详情 JSON 的 tag_list 提取分类目录段。两种形态：
// - national_lesson 等：标签带 tag_dimension_id，按已知维度排序
// - special_edu 等：维度为 null，按名称特征归桶（学段/学科/年级/册次）后排序
fn extract_category_path(detail: &Value) -> Vec<String> {
    let Some(tags) = detail.get("tag_list").and_then(Value::as_array) else {
        return Vec::new();
    };
    let name_of = |t: &Value| {
        t.get("tag_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    let mut path: Vec<String> = Vec::new();
    let push_unique = |path: &mut Vec<String>, name: String| {
        if !path.contains(&name) {
            path.push(name);
        }
    };

    let dimensioned: Vec<(String, String)> = tags
        .iter()
        .filter_map(|t| {
            let dim = t.get("tag_dimension_id").and_then(Value::as_str)?;
            Some((dim.to_string(), name_of(t)?))
        })
        .collect();

    if !dimensioned.is_empty() {
        for dim in TAG_DIMENSION_ORDER {
            for (d, name) in &dimensioned {
                if d == dim {
                    push_unique(&mut path, name.clone());
                }
            }
        }
        return path;
    }

    let mut stages = Vec::new();
    let mut grades = Vec::new();
    let mut volumes = Vec::new();
    let mut subjects = Vec::new();
    for name in tags.iter().filter_map(name_of) {
        if matches!(name.as_str(), "小学" | "初中" | "高中") {
            stages.push(name);
        } else if name.ends_with("年级")
            || matches!(name.as_str(), "高一" | "高二" | "高三" | "初一" | "初二" | "初三")
        {
            grades.push(name);
        } else if name.ends_with('册') {
            volumes.push(name);
        } else {
            subjects.push(name);
        }
    }
    for group in [stages, subjects, grades, volumes] {
        for name in group {
            push_unique(&mut path, name);
        }
    }
    path
}

/// 解析课程页 URL，返回其下所有可下载资源。
#[tauri::command]
pub async fn parse_course_url(url: String) -> Result<CourseParseResult, String> {
    let parsed = Url::parse(url.trim()).map_err(|e| format!("无效的链接: {e}"))?;
    let route = resolve_route(&parsed).ok_or_else(|| {
        "暂不支持该链接类型，请粘贴课程/视频/课件页面的地址".to_string()
    })?;

    log::info!("解析课程详情: {}", route.detail_url);
    let detail: Value = http::get_json(&route.detail_url)
        .await
        .map_err(|e| format!("获取课程详情失败: {e}"))?;

    let course_title = detail
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("未命名课程")
        .to_string();

    let mut resources = Vec::new();
    if route.relation_keys.is_empty() {
        // 资源在顶层 ti_items
        if let Some(res) = extract_resource(&detail, &course_title) {
            resources.push(res);
        }
    } else {
        let relations = detail.get("relations");
        for key in &route.relation_keys {
            let Some(arr) = relations.and_then(|r| r.get(*key)).and_then(Value::as_array) else {
                continue;
            };
            for item in arr {
                if let Some(res) = extract_resource(item, &course_title) {
                    resources.push(res);
                }
            }
        }
    }

    if resources.is_empty() {
        return Err("未找到可下载的资源（部分课件需要登录鉴权，暂未支持）".to_string());
    }

    Ok(CourseParseResult {
        title: course_title,
        category_path: extract_category_path(&detail),
        resources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 样例取自线上真实详情 JSON（2026-07）
    #[test]
    fn category_path_with_dimension_ids() {
        // national_lesson：标签带维度，bklx（课程包）不入目录
        let detail = json!({
            "tag_list": [
                {"tag_name": "课程包", "tag_dimension_id": "bklx"},
                {"tag_name": "人教版", "tag_dimension_id": "zxxbb"},
                {"tag_name": "必修 全一册", "tag_dimension_id": "zxxcc"},
                {"tag_name": "高中", "tag_dimension_id": "zxxxd"},
                {"tag_name": "体育与健康", "tag_dimension_id": "zxxxk"},
            ]
        });
        assert_eq!(
            extract_category_path(&detail),
            vec!["高中", "体育与健康", "人教版", "必修 全一册"]
        );
    }

    #[test]
    fn category_path_without_dimension_ids() {
        // special_edu（基础作业）：维度为 null，按名称特征排成 学科/年级/册次
        let detail = json!({
            "tag_list": [
                {"tag_name": "三年级", "tag_dimension_id": null},
                {"tag_name": "上册", "tag_dimension_id": null},
                {"tag_name": "语文", "tag_dimension_id": null},
            ]
        });
        assert_eq!(extract_category_path(&detail), vec!["语文", "三年级", "上册"]);
    }

    #[test]
    fn category_path_missing_tags() {
        assert!(extract_category_path(&json!({})).is_empty());
        assert!(extract_category_path(&json!({"tag_list": []})).is_empty());
    }
}
