use crate::http;
use crate::models::{CourseParseResult, CourseResource};
use serde_json::Value;
use url::Url;

// 平台各类课程页 URL → 详情接口的映射。绝大多数详情 JSON 的形态一致：
// 要么顶层直接带 ti_items（单个资源），要么带 relations（命名的资源数组）。
// 参考 52beijixing/smartedu-download 的分发表，用当前线上接口校准。

fn collect_into(arr: &[Value], fallback_title: &str, out: &mut Vec<CourseResource>) {
    for item in arr {
        if let Some(res) = extract_resource(item, fallback_title) {
            out.push(res);
        }
    }
}

// 遍历 relations 下所有数组。键序排序，保证多次解析的资源顺序稳定。
fn collect_all_relations(detail: &Value, fallback_title: &str, out: &mut Vec<CourseResource>) {
    let Some(map) = detail.get("relations").and_then(Value::as_object) else {
        return;
    };
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(arr) = map.get(key).and_then(Value::as_array) {
            collect_into(arr, fallback_title, out);
        }
    }
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

// 详情 JSON 里资源的存放位置
enum Source {
    // 资源就在顶层 ti_items
    TopLevel,
    // 资源在 relations 下的指定键里
    Relations(&'static [&'static str]),
    // 形态不定（resourceType 可变、relations 的键跟随 classHourId），遍历所有 relations
    AllRelations,
}

// 详情接口地址 + 资源位置
struct Route {
    detail_url: String,
    source: Source,
}

// 依据课程页 URL 推断详情接口。返回 None 表示暂不支持的链接类型。
fn resolve_route(url: &Url) -> Option<Route> {
    let path = url.path();
    let host = url.host_str().unwrap_or("");

    // 同步课堂 - 通用资源详情页。resourceType 即详情接口的资源桶名（如
    // knowledge_micro_lesson_package = 知识点微课包），relations 的键跟随
    // classHourId（lesson_1/lesson_2…），故遍历所有 relations。
    // 注意要放在 /syncClassroom/prepare/detail 之前判断会误伤，故只匹配精确前缀。
    if path.starts_with("/syncClassroom/detail") {
        let id = query_param(url, "resourceId")?;
        let bucket = query_param(url, "resourceType").filter(|t| is_safe_segment(t))?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/{bucket}/resources/details/{id}.json"
            ),
            source: Source::AllRelations,
        });
    }

    // 同步课堂 - 课程视频（书课包）
    if path.starts_with("/syncClassroom/classActivity") {
        let id = query_param(url, "activityId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-2.ykt.cbern.com.cn/zxx/ndrv2/national_lesson/resources/details/{id}.json"
            ),
            source: Source::Relations(&["national_course_resource"]),
        });
    }

    // 同步课堂 - 一师一优课（含教学设计、课堂实录视频、教学素材）
    if path.starts_with("/syncClassroom/prepare/detail") {
        if let Some(id) = query_param(url, "lessonId") {
            return Some(Route {
                detail_url: format!(
                    "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/prepare_lesson/resources/details/{id}.json"
                ),
                source: Source::Relations(&[
                    "lesson_plan_design",
                    "classroom_record",
                    "teaching_assets",
                ]),
            });
        }
        // 备课资源（单个课件）
        let id = query_param(url, "resourceId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-2.ykt.cbern.com.cn/zxx/ndrv2/prepare_sub_type/resources/details/{id}.json"
            ),
            source: Source::TopLevel,
        });
    }

    // 同步课堂 - 实验课
    if path.starts_with("/syncClassroom/experimentLesson") {
        let id = query_param(url, "courseId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/experiment/resources/details/{id}.json"
            ),
            source: Source::Relations(&["lesson_1", "experiment_video"]),
        });
    }

    // 同步课堂 - 基础作业（文档）
    if path.starts_with("/syncClassroom/basicWork/detail") {
        let id = query_param(url, "contentId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/special_edu/resources/details/{id}.json"
            ),
            source: Source::TopLevel,
        });
    }

    // 学科精品课
    if path.starts_with("/qualityCourse") {
        let id = query_param(url, "courseId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/resources/{id}.json"
            ),
            source: Source::Relations(&["course_resource"]),
        });
    }

    // 基础教育精品课（jpk 子站，年度评优课）
    if path.starts_with("/yearQualityCourse") || host.starts_with("jpk.") {
        let id = query_param(url, "courseId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/competitive/elite_lesson/resources/{id}.json"
            ),
            source: Source::Relations(&["course_resource"]),
        });
    }

    // 德育/思政视频
    if path.starts_with("/sedu/detail") {
        let id = query_param(url, "contentId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/zxx/ndrs/special_edu/resources/details/{id}.json"
            ),
            source: Source::TopLevel,
        });
    }

    // 智慧教育视频
    if path.starts_with("/wisdom/detail") {
        let id = query_param(url, "contentId")?;
        return Some(Route {
            detail_url: format!(
                "https://s-file-1.ykt.cbern.com.cn/ldjy/ndrs/special_edu/resources/details/{id}.json"
            ),
            source: Source::TopLevel,
        });
    }

    None
}

// resourceType 直接拼进接口路径，限定为小写字母/数字/下划线，防止拼出越权路径
fn is_safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
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

    // 标题优先取 title（"课件"/"教学设计" 这类资源自身的名字）。
    // global_title 不可靠：书课包里它是整节课的名字（几个资源全同名，会互相覆盖），
    // 微课包里它是带扩展名的文件名（会拼出 "X.mp4.mp4"）。
    let title = obj
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            obj.get("global_title")
                .and_then(|g| g.get("zh-CN"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or(fallback_title);
    let title = strip_known_extension(title).to_string();

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

    // 视频优先走 m3u8；声明是视频却没有 m3u8（少数直链 mp4）时退回普通文件下载，
    // 否则整条资源会被静默丢弃。
    let (download_url, item_format, is_video) = if has_m3u8 {
        (pick_video_url(ti_items)?, String::new(), true)
    } else {
        let item = pick_file_item(ti_items)?;
        let fmt = item
            .get("ti_format")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        (first_storage(item)?, fmt, false)
    };

    // 关键：扩展名以实际下载到的文件为准，而不是 custom_properties.format。
    // 平台上 format 标 docx/pptx 的课件，ti_items 里往往只有转码后的 pdf.pdf，
    // 按 docx 存盘会得到一个「打不开」的假 Word 文件。
    let out_format = if is_video {
        "mp4".to_string()
    } else {
        [item_format, url_extension(&download_url).unwrap_or_default(), format]
            .into_iter()
            .find(|f| is_plausible_extension(f))
            .unwrap_or_else(|| "bin".to_string())
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

// ti_items 里除正文外还混着缩略图、AI 字幕/摘要、白板工程等附属项，兜底时要跳过
fn is_attachment(item: &Value) -> bool {
    let flag = item
        .get("ti_file_flag")
        .and_then(Value::as_str)
        .unwrap_or("");
    let format = item.get("ti_format").and_then(Value::as_str).unwrap_or("");
    format == "folder"
        || format == "superboard"
        || flag.starts_with("thumbnail")
        || flag.starts_with("ai_")
        || matches!(flag, "image" | "preview" | "remarks" | "superboard")
}

// 从 ti_items 里挑课件正文项：优先 ti_file_flag=="href"，其次 "source"
// （基础作业等 special_edu 文档用 source 标源 PDF），再次 "pdf"（课件/教学设计
// 只提供转码后的 PDF），最后兜底取第一个非附属项。
fn pick_file_item(ti_items: &[Value]) -> Option<&Value> {
    let flagged = |v: &str| {
        ti_items
            .iter()
            .find(|it| it.get("ti_file_flag").and_then(Value::as_str) == Some(v))
    };
    flagged("href")
        .or_else(|| flagged("source"))
        .or_else(|| flagged("pdf"))
        .or_else(|| ti_items.iter().find(|it| !is_attachment(it)))
}

// 扩展名合理性检查：排除空值、folder/superboard 这类非扩展名的类型标记
fn is_plausible_extension(s: &str) -> bool {
    !s.is_empty() && s.len() <= 5 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

// 标题里可能自带扩展名（微课包的 "有理数 电教馆.mp4"），去掉以免存成 "X.mp4.mp4"。
// 只认已知扩展名，避免误伤 "1.2.1 有理数的概念" 这类带点的正常标题。
const KNOWN_EXTENSIONS: &[&str] = &[
    "mp4", "m3u8", "avi", "flv", "mov", "mkv", "wmv", "mp3", "wav", "pdf", "doc", "docx", "ppt",
    "pptx", "xls", "xlsx", "txt", "zip", "rar", "7z", "jpg", "jpeg", "png", "gif",
];

fn strip_known_extension(title: &str) -> &str {
    match title.rsplit_once('.') {
        Some((stem, ext))
            if !stem.trim().is_empty() && KNOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()) =>
        {
            stem.trim_end()
        }
        _ => title,
    }
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

// 按路由声明的位置收集资源，并做去重/重名消歧。
fn collect_resources(detail: &Value, source: &Source, course_title: &str) -> Vec<CourseResource> {
    let mut resources = Vec::new();

    match source {
        Source::TopLevel => {
            if let Some(res) = extract_resource(detail, course_title) {
                resources.push(res);
            }
        }
        Source::Relations(keys) => {
            let relations = detail.get("relations");
            for key in *keys {
                if let Some(arr) = relations.and_then(|r| r.get(*key)).and_then(Value::as_array) {
                    collect_into(arr, course_title, &mut resources);
                }
            }
        }
        Source::AllRelations => {
            // 顶层 ti_items 非空时也可能直接挂着资源
            if let Some(res) = extract_resource(detail, course_title) {
                resources.push(res);
            }
            collect_all_relations(detail, course_title, &mut resources);
        }
    }

    // 硬编码的 relation 键会随平台调整而失效（实验课就从 lesson_1 变成了
    // experiment_work_information），一无所获时退回遍历所有 relations。
    if resources.is_empty() {
        log::warn!("预期的 relation 键未命中，改为遍历全部 relations");
        collect_all_relations(detail, course_title, &mut resources);
    }

    // 同一资源可能在顶层和 relations 里各出现一次，按下载地址去重
    let mut seen = std::collections::HashSet::new();
    resources.retain(|r| seen.insert(r.download_url.clone()));

    // 同一课程的资源都存进同一目录，标题+格式相同会写到同一个文件：
    // 并发下载时互相覆盖甚至写坏。重名的追加序号。
    let mut used = std::collections::HashSet::new();
    for res in &mut resources {
        let key = |t: &str, f: &str| format!("{}.{}", t.to_lowercase(), f.to_lowercase());
        if used.insert(key(&res.title, &res.format)) {
            continue;
        }
        for n in 2.. {
            let candidate = format!("{} ({n})", res.title);
            if used.insert(key(&candidate, &res.format)) {
                res.title = candidate;
                break;
            }
        }
    }

    resources
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

    let resources = collect_resources(&detail, &route.source, &course_title);

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

    // 平台上 format 标 docx 的「教学设计」实际只提供转码 PDF，
    // 必须按 pdf 存盘，否则得到一个打不开的假 Word 文件。
    #[test]
    fn declared_docx_with_only_pdf_saves_as_pdf() {
        let obj = json!({
            "id": "x",
            "title": "教学设计",
            "custom_properties": {"format": "docx"},
            "ti_items": [
                {"ti_file_flag": "image", "ti_format": "folder", "ti_storages": ["https://h/a/image"]},
                {"ti_file_flag": "pdf", "ti_format": "pdf", "ti_storages": ["https://h/a/transcode/pdf.pdf"]},
                {"ti_file_flag": "thumbnail_1", "ti_format": "jpg", "ti_storages": ["https://h/a/1.jpg"]},
            ]
        });
        let res = extract_resource(&obj, "课").unwrap();
        assert_eq!(res.format, "pdf");
        assert_eq!(res.download_url, "https://h/a/transcode/pdf.pdf");
        assert!(!res.is_video);
    }

    // 有真源文件时（source 标 docx）仍按真实扩展名存
    #[test]
    fn real_source_document_keeps_its_format() {
        let obj = json!({
            "id": "x",
            "title": "作业",
            "custom_properties": {"format": "docx"},
            "ti_items": [
                {"ti_file_flag": "source", "ti_format": "docx", "ti_storages": ["https://h/a/b.docx"]},
                {"ti_file_flag": "pdf", "ti_format": "pdf", "ti_storages": ["https://h/a/pdf.pdf"]},
            ]
        });
        let res = extract_resource(&obj, "课").unwrap();
        assert_eq!(res.format, "docx");
        assert_eq!(res.download_url, "https://h/a/b.docx");
    }

    // 视频仍走 m3u8 流程，优先 720p
    #[test]
    fn video_prefers_720p_m3u8() {
        let obj = json!({
            "id": "v",
            "title": "微课视频",
            "custom_properties": {"format": "mp4"},
            "ti_items": [
                {"ti_file_flag": "href", "ti_format": "m3u8", "ti_storages": ["https://h/v/full.m3u8"]},
                {"ti_file_flag": "href-720p-m3u8", "ti_format": "m3u8", "ti_storages": ["https://h/v/720.m3u8"]},
                {"ti_file_flag": "ai_caption", "ti_format": "srt", "ti_storages": ["https://h/v/c.srt"]},
            ]
        });
        let res = extract_resource(&obj, "课").unwrap();
        assert!(res.is_video);
        assert_eq!(res.format, "mp4");
        assert_eq!(res.download_url, "https://h/v/720.m3u8");
    }

    // 声明是视频却没有 m3u8（直链 mp4）时不再被丢弃
    #[test]
    fn declared_video_without_m3u8_falls_back_to_direct_file() {
        let obj = json!({
            "id": "v2",
            "title": "视频",
            "custom_properties": {"format": "mp4"},
            "ti_items": [
                {"ti_file_flag": "href", "ti_format": "mp4", "ti_storages": ["https://h/v/a.mp4"]},
            ]
        });
        let res = extract_resource(&obj, "课").unwrap();
        assert!(!res.is_video);
        assert_eq!(res.format, "mp4");
    }

    // 只有附属项（缩略图/白板/目录）时不产出资源，避免下载出垃圾文件
    #[test]
    fn attachments_only_yields_nothing() {
        let obj = json!({
            "id": "x",
            "title": "空",
            "custom_properties": {"format": "pptx"},
            "ti_items": [
                {"ti_file_flag": "thumbnail", "ti_format": "folder", "ti_storages": ["https://h/a/image"]},
                {"ti_file_flag": "superboard", "ti_format": "superboard", "ti_storages": ["https://h/a/d.superboard"]},
            ]
        });
        assert!(extract_resource(&obj, "课").is_none());
    }

    #[test]
    fn resolves_micro_lesson_package_detail_route() {
        let url = Url::parse(
            "https://basic.smartedu.cn/syncClassroom/detail?resourceId=d5afd160-fefc-47e5-a500-fb42804f7df5&resourceType=knowledge_micro_lesson_package&classHourId=lesson_1",
        )
        .unwrap();
        let route = resolve_route(&url).expect("应支持 /syncClassroom/detail");
        assert_eq!(
            route.detail_url,
            "https://s-file-1.ykt.cbern.com.cn/zxx/ndrv2/knowledge_micro_lesson_package/resources/details/d5afd160-fefc-47e5-a500-fb42804f7df5.json"
        );
        assert!(matches!(route.source, Source::AllRelations));
    }

    // resourceType 会拼进接口路径，必须拒绝可越权的取值
    #[test]
    fn rejects_unsafe_resource_type() {
        let url = Url::parse(
            "https://basic.smartedu.cn/syncClassroom/detail?resourceId=abc&resourceType=..%2F..%2Fetc",
        )
        .unwrap();
        assert!(resolve_route(&url).is_none());
    }

    #[test]
    fn prepare_detail_route_still_matches() {
        let url = Url::parse(
            "https://basic.smartedu.cn/syncClassroom/prepare/detail?lessonId=abc",
        )
        .unwrap();
        let route = resolve_route(&url).unwrap();
        assert!(route.detail_url.contains("prepare_lesson"));
        assert!(matches!(route.source, Source::Relations(_)));
    }

    // 书课包里几个资源的 global_title 都是整节课的名字，取它会让 4 份文档同名互相覆盖
    #[test]
    fn prefers_specific_title_over_shared_global_title() {
        let obj = json!({
            "id": "x",
            "title": "教学设计",
            "global_title": {"zh-CN": "1.2.1 有理数的概念"},
            "custom_properties": {"format": "docx"},
            "ti_items": [
                {"ti_file_flag": "pdf", "ti_format": "pdf", "ti_storages": ["https://h/a/pdf.pdf"]},
            ]
        });
        assert_eq!(extract_resource(&obj, "课").unwrap().title, "教学设计");
    }

    #[test]
    fn strips_extension_already_in_title() {
        assert_eq!(strip_known_extension("有理数 电教馆.mp4"), "有理数 电教馆");
        assert_eq!(strip_known_extension("讲义.PDF"), "讲义");
        // 带点但不是扩展名的正常标题不能被截断
        assert_eq!(strip_known_extension("1.2.1 有理数的概念"), "1.2.1 有理数的概念");
        assert_eq!(strip_known_extension("第一章 1.2"), "第一章 1.2");
        assert_eq!(strip_known_extension(".mp4"), ".mp4");
    }

    fn doc_item(title: &str, format: &str, url: &str) -> Value {
        json!({
            "id": url,
            "title": title,
            "custom_properties": {"format": format},
            "ti_items": [
                {"ti_file_flag": "pdf", "ti_format": "pdf", "ti_storages": [url]},
            ]
        })
    }

    // 实验课的 relations 键是 experiment_work_information，不在路由硬编码的键里；
    // 一无所获时应退回遍历全部 relations，而不是报「获取失败」
    #[test]
    fn falls_back_to_all_relations_when_declared_keys_miss() {
        let detail = json!({
            "title": "辨认简单的立体图形",
            "relations": {
                "experiment_work_information": [
                    doc_item("实验教学视频", "mp4", "https://h/v/a.pdf"),
                    doc_item("课件", "pptx", "https://h/p/b.pdf"),
                ]
            }
        });
        let source = Source::Relations(&["lesson_1", "experiment_video"]);
        let resources = collect_resources(&detail, &source, "实验课");
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].title, "实验教学视频");
        assert_eq!(resources[1].title, "课件");
    }

    // 同名同格式的资源会写到同一路径，并发下载互相写坏，必须消歧
    #[test]
    fn duplicate_titles_get_numbered() {
        let detail = json!({
            "relations": {
                "national_course_resource": [
                    doc_item("课件", "pdf", "https://h/a.pdf"),
                    doc_item("课件", "pdf", "https://h/b.pdf"),
                    doc_item("课件", "pdf", "https://h/c.pdf"),
                ]
            }
        });
        let source = Source::Relations(&["national_course_resource"]);
        let titles: Vec<String> = collect_resources(&detail, &source, "课")
            .into_iter()
            .map(|r| r.title)
            .collect();
        assert_eq!(titles, vec!["课件", "课件 (2)", "课件 (3)"]);
    }

    // 同一资源在顶层和 relations 里各挂一次时只保留一份
    #[test]
    fn same_download_url_is_deduped() {
        let mut top = doc_item("微课视频", "mp4", "https://h/only.pdf");
        top["relations"] = json!({"lesson_1": [doc_item("微课视频", "mp4", "https://h/only.pdf")]});
        let resources = collect_resources(&top, &Source::AllRelations, "课");
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].title, "微课视频");
    }
}
