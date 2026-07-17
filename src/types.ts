export interface DropdownOption {
  value: string;
  label: string;
}

export interface Textbook {
  id: string;
  cover_url: string;
  title: string;
  total_uv: number;
  like_count: number;
  download_url: string;
}

// 当前各级筛选选中项的显示名，用于「按分类保存」的目录结构
export interface TextbookLabels {
  category: string;
  subject: string;
  version: string;
  grade: string;
  year: string;
}

// 后端解析课程 URL 得到的单个资源（视频或课件）
export interface CourseResource {
  id: string;
  title: string;
  format: string;
  download_url: string;
  is_video: boolean;
  cover_url: string;
}

// 一个课程 URL 的解析结果
export interface CourseParseResult {
  title: string;
  // 分类目录段（学段/学科/版本/年级/册次，可为空），用于卡片展示与「按分类保存」
  category_path: string[];
  resources: CourseResource[];
}
