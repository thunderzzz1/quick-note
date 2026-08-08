import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import { renderMarkdown } from '../../lib/markdown';

export function NoteDetail(props: { id: string }) {
  const [html, setHtml] = useState('');

  useEffect(() => {
    void (async () => {
      const md = await api.getNote(props.id);
      if (!md) return;
      const baseDir = await api.getDataDir();
      setHtml(await renderMarkdown(md, baseDir));
    })();
  }, [props.id]);

  return (
    <article
      className="flex-1 overflow-y-auto p-6 prose prose-sm max-w-none"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
