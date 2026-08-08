import { invoke } from '@tauri-apps/api/core';
import type {
  Category,
  NoteMeta,
  PastedImage,
  SaveNoteResult,
  Suggestion,
} from '../types';

export const api = {
  saveNote: (markdown: string, images: PastedImage[]) =>
    invoke<SaveNoteResult>('save_note', { markdown, images }),
  listNotes: (date?: string) => invoke<NoteMeta[]>('list_notes', { date: date ?? null }),
  listNotesByCategory: (categoryId: number) =>
    invoke<NoteMeta[]>('list_notes_by_category', { categoryId }),
  getDataDir: () => invoke<string>('get_data_dir'),
  getNote: (id: string) => invoke<string | null>('get_note', { id }),
  updateNote: (id: string, markdown: string) => invoke<void>('update_note', { id, markdown }),
  saveImage: (noteId: string, image: PastedImage) =>
    invoke<string>('save_image', { noteId, image }),
  rebuildIndex: () => invoke<[number, number]>('rebuild_index'),
  listCategories: () => invoke<Category[]>('list_categories'),
  pendingSuggestionCount: () => invoke<number>('pending_suggestion_count'),
  listSuggestions: (date: string) => invoke<Suggestion[]>('list_suggestions', { date }),
  acceptSuggestion: (
    id: number,
    categoryName?: string,
    summary?: string,
    keywords?: string,
  ) =>
    invoke<void>('accept_suggestion', {
      suggestionId: id,
      categoryName: categoryName ?? null,
      summary: summary ?? null,
      keywords: keywords ?? null,
    }),
  skipSuggestion: (id: number) => invoke<void>('skip_suggestion', { suggestionId: id }),
  acceptAll: (date: string) => invoke<number>('accept_all', { date }),
};
