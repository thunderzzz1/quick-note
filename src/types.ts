export type AiStatus = 'pending' | 'suggested' | 'confirmed' | 'skipped' | 'failed';

export interface NoteMeta {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  ai_status: AiStatus;
  category_id: number | null;
  summary: string | null;
  keywords: string | null; // JSON 数组字符串
  tags: string | null;
}

export interface PastedImage {
  filename: string;
  mime: string;
  bytes: number[];
}

export interface SaveNoteResult {
  id: string;
  markdown_path: string;
  image_refs: string[];
}

export interface Category {
  id: number;
  name: string;
  origin: 'builtin' | 'ai' | 'user';
  enabled: boolean;
  sort_order: number;
  created_at: string;
}
