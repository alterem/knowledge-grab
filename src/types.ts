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

// 目录页里的一节课。url 是该课时的详情页地址，交给 parse_course_lessons 批量解析
export interface CatalogLesson {
  id: string;
  title: string;
  // 形如「第一单元 时、分、秒 / 秒的认识」
  chapter: string;
  url: string;
  resource_type: string;
}

// 一册教材及其全部课时
export interface CatalogBook {
  id: string;
  title: string;
  category_path: string[];
  lessons: CatalogLesson[];
}

// 同步课堂目录页（列表页 / 整册教材页）的解析结果
export interface CourseCatalog {
  title: string;
  category_path: string[];
  books: CatalogBook[];
}

// parse_course_link 的返回值：单个课时页给资源清单，目录页给课时目录
export type CourseLink =
  | ({ kind: 'course' } & CourseParseResult)
  | ({ kind: 'catalog' } & CourseCatalog);

// 批量解析进度事件 course-parse-progress 的载荷
export interface CourseParseProgress {
  done: number;
  total: number;
  title: string;
}
