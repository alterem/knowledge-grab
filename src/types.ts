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
