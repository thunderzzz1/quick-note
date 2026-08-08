import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { Category, NoteMeta } from '../../types';
import { NoteDetail } from '../main/NoteDetail';

export function KnowledgeBase() {
  const [notes, setNotes] = useState<NoteMeta[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [q, setQ] = useState('');

  useEffect(() => {
    void (async () => {
      const cats = await api.listCategories();
      const kb = cats.find((c: Category) => c.name === '知识库');
      const all = await api.listNotes();
      const inKb = kb ? all.filter((n) => n.category_id === kb.id) : [];
      setNotes(inKb);
      if (inKb[0]) setSelected(inKb[0].id);
    })();
  }, []);

  return (
    <div className="flex flex-1 overflow-hidden">
      <div className="w-60 border-r border-gray-200 flex flex-col">
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="搜索知识…"
          className="m-3 border border-gray-200 rounded-md px-3 py-1.5 text-sm"
        />
        <div className="overflow-y-auto flex-1">
          {notes
            .filter((n) => !q || n.title.includes(q))
            .map((n) => (
              <button
                key={n.id}
                onClick={() => setSelected(n.id)}
                className={`w-full text-left px-4 py-2.5 border-b border-gray-100 ${
                  selected === n.id ? 'bg-blue-50' : 'hover:bg-gray-50'
                }`}
              >
                <div className="text-sm text-gray-800">{n.title}</div>
                <div className="text-xs text-gray-400">{n.created_at.slice(0, 10)}</div>
              </button>
            ))}
        </div>
      </div>
      {selected ? (
        <NoteDetail id={selected} />
      ) : (
        <div className="flex-1 grid place-items-center text-gray-400">选择一条知识</div>
      )}
    </div>
  );
}
