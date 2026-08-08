import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { NoteMeta } from '../../types';

export function NoteList(props: {
  date?: string;
  categoryId?: number;
  onOpen: (id: string) => void;
}) {
  const [notes, setNotes] = useState<NoteMeta[]>([]);
  const [q, setQ] = useState('');

  useEffect(() => {
    void (async () => {
      const rows = props.categoryId
        ? await api.listNotesByCategory(props.categoryId)
        : await api.listNotes(props.date);
      setNotes(rows);
    })();
  }, [props.date, props.categoryId]);

  return (
    <div className="flex-1 overflow-y-auto flex flex-col">
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="搜索记录…"
        className="m-3 border border-gray-200 rounded-md px-3 py-1.5 text-sm"
      />
      {notes
        .filter((n) => !q || n.title.includes(q))
        .map((n) => (
          <button
            key={n.id}
            onClick={() => props.onOpen(n.id)}
            className="w-full text-left px-4 py-3 border-b border-gray-100 hover:bg-gray-50"
          >
            <div className="text-sm font-medium text-gray-800">{n.title}</div>
            <div className="text-xs text-gray-400">
              {n.created_at.slice(0, 16).replace('T', ' ')} · {n.ai_status}
            </div>
            {n.summary ? <div className="text-xs text-gray-500 mt-1">{n.summary}</div> : null}
          </button>
        ))}
    </div>
  );
}
