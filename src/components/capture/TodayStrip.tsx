import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { NoteMeta } from '../../types';

export function TodayStrip({ refreshKey }: { refreshKey?: number }) {
  const [expanded, setExpanded] = useState(false);
  const [notes, setNotes] = useState<NoteMeta[]>([]);

  useEffect(() => {
    const today = new Date().toISOString().slice(0, 10);
    void api.listNotes(today).then(setNotes);
  }, [refreshKey]);

  return (
    <div className="border-t border-gray-200 bg-white">
      <button
        type="button"
        className="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-600 hover:bg-gray-50"
        onClick={() => setExpanded((v) => !v)}
      >
        <span>📥 今日 {notes.length} 条</span>
        <span className="ml-auto">{expanded ? '▴' : '▾'}</span>
      </button>
      {expanded && (
        <ul className="max-h-40 overflow-y-auto border-t border-gray-100">
          {notes.map((n) => (
            <li key={n.id} className="px-3 py-2 text-sm text-gray-700 hover:bg-gray-50">
              {n.title} <span className="text-gray-400 text-xs">{n.created_at.slice(11, 16)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
