import type { Suggestion } from '../../types';

export function SuggestionCard(props: {
  suggestion: Suggestion;
  original: string;
  onAccept: (s: Suggestion) => void;
  onSkip: (s: Suggestion) => void;
}) {
  const s = props.suggestion;
  return (
    <div className="border border-gray-200 rounded-lg p-3 mb-2">
      <div className="text-xs text-gray-400">原文：{props.original}</div>
      <div className="text-sm font-medium mt-1">{s.summary ?? s.ai_category ?? '（无摘要）'}</div>
      {s.keywords ? (
        <div className="flex gap-1 mt-1">
          {(JSON.parse(s.keywords) as string[]).map((k) => (
            <span key={k} className="bg-gray-100 rounded px-1.5 text-xs text-gray-600">
              {k}
            </span>
          ))}
        </div>
      ) : null}
      {s.new_category_proposal ? (
        <div className="mt-1 text-xs text-amber-700 bg-amber-50 border border-dashed border-amber-300 rounded px-2 py-1">
          🆕 建议新分类「{s.new_category_proposal}」
        </div>
      ) : null}
      <div className="flex gap-2 mt-2">
        <button
          onClick={() => props.onAccept(s)}
          className="bg-blue-50 text-blue-700 rounded px-3 py-1 text-sm"
        >
          ✓ 接受
        </button>
        <button onClick={() => props.onSkip(s)} className="text-gray-400 px-2 py-1 text-sm">
          跳过
        </button>
      </div>
    </div>
  );
}
