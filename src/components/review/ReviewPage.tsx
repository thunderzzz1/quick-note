import { useEffect, useMemo, useState } from 'react';
import { api } from '../../lib/tauri';
import type { Suggestion } from '../../types';
import { SuggestionCard } from './SuggestionCard';

export function ReviewPage({ onDone }: { onDone?: () => void }) {
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [originals, setOriginals] = useState<Record<string, string>>({});
  const [running, setRunning] = useState(false);
  const [msg, setMsg] = useState('');
  const today = new Date().toISOString().slice(0, 10);

  const load = async () => {
    const list = await api.listSuggestions(today);
    setSuggestions(list.filter((s) => s.status === 'suggested'));
    const map: Record<string, string> = {};
    for (const s of list) {
      map[s.note_id] = (await api.getNote(s.note_id)) ?? '';
    }
    setOriginals(map);
  };

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const groups = useMemo(() => {
    const g = new Map<string, Suggestion[]>();
    for (const s of suggestions) {
      const key = s.new_category_proposal ?? s.ai_category ?? '其他';
      g.set(key, [...(g.get(key) ?? []), s]);
    }
    return [...g.entries()];
  }, [suggestions]);

  const accept = async (s: Suggestion) => {
    await api.acceptSuggestion(
      s.id,
      s.ai_category ?? undefined,
      s.summary ?? undefined,
      s.keywords ?? undefined,
    );
    await load();
    onDone?.();
  };

  const skip = async (s: Suggestion) => {
    await api.skipSuggestion(s.id);
    await load();
    onDone?.();
  };

  const runNow = async () => {
    setRunning(true);
    setMsg('');
    try {
      const result = await api.runAiOrg();
      setMsg(
        result.failed.length > 0
          ? `完成：建议 ${result.suggested} 条，失败 ${result.failed.length} 条`
          : `完成：处理 ${result.processed} 条，建议 ${result.suggested} 条`,
      );
      await load();
      onDone?.();
    } catch (e) {
      setMsg(`整理失败：${String(e)}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="flex items-center gap-3 mb-3">
        <h2 className="text-base font-bold">{today} 整理建议</h2>
        <span className="text-xs text-gray-400">共 {suggestions.length} 条</span>
        <button
          className="ml-auto bg-blue-500 text-white rounded px-3 py-1 text-sm"
          onClick={async () => {
            await api.acceptAll(today);
            await load();
            onDone?.();
          }}
        >
          全部接受
        </button>
        <button
          onClick={() => void runNow()}
          disabled={running}
          className="bg-gray-100 text-gray-700 rounded px-3 py-1 text-sm disabled:opacity-50"
        >
          {running ? '整理中…' : '立即整理'}
        </button>
      </div>
      {msg ? <div className="text-sm text-gray-500 mb-3">{msg}</div> : null}
      {groups.map(([cat, list]) => (
        <section key={cat} className="mb-4">
          <h3 className="text-sm text-gray-600 font-medium mb-2">
            {cat}（{list.length}）
          </h3>
          {list.map((s) => (
            <SuggestionCard
              key={s.id}
              suggestion={s}
              original={originals[s.note_id] ?? ''}
              onAccept={accept}
              onSkip={skip}
            />
          ))}
        </section>
      ))}
      {suggestions.length === 0 ? (
        <div className="text-gray-400 text-sm mt-10 text-center">
          今天的记录都已整理完毕 🎉
        </div>
      ) : null}
    </div>
  );
}
