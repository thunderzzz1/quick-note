import { convertFileSrc } from '@tauri-apps/api/core';
import DOMPurify from 'dompurify';
import { marked } from 'marked';

export async function renderMarkdown(md: string, baseDir: string): Promise<string> {
  const raw = await marked.parse(md, { async: false });
  const sanitized = DOMPurify.sanitize(raw);
  return sanitized.replace(
    /src="attachments\/([^"]+)"/g,
    (_match, rel: string) => `src="${convertFileSrc(`${baseDir}/attachments/${rel}`)}"`,
  );
}
