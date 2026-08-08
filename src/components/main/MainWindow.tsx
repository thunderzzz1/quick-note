import { useEffect, useState } from 'react';
import { api } from '../../lib/tauri';
import type { Category } from '../../types';
import { KnowledgeBase } from '../kb/KnowledgeBase';
import { ReviewPage } from '../review/ReviewPage';
import { SettingsPage } from '../settings/SettingsPage';
import { NoteDetail } from './NoteDetail';
import { NoteList } from './NoteList';
import { Sidebar, type SidebarSection } from './Sidebar';

const CATEGORY_ICONS: Record<string, string> = {
  待办: '☑',
  进度: '↗',
  提醒: '⏰',
  知识库: '📚',
  其他: '📄',
};

export function MainWindow() {
  const [active, setActive] = useState('inbox');
  const [reviewCount, setReviewCount] = useState(0);
  const [categories, setCategories] = useState<Category[]>([]);
  const [openNote, setOpenNote] = useState<string | null>(null);

  const refresh = () => {
    void api.pendingSuggestionCount().then(setReviewCount);
    void api.listCategories().then(setCategories);
  };

  useEffect(() => {
    refresh();
  }, [active]);

  const sections: SidebarSection[] = [
    { key: 'inbox', label: '今日速记', icon: '📥' },
    { key: 'review', label: '整理建议', icon: '✨', badge: reviewCount },
    ...categories
      .filter((c) => c.name !== '知识库')
      .map((c) => ({
        key: `cat-${c.id}`,
        label: c.name,
        icon: CATEGORY_ICONS[c.name] ?? '•',
      })),
    { key: 'kb', label: '知识库', icon: '📚' },
    { key: 'settings', label: '设置', icon: '⚙' },
  ];

  const select = (key: string) => {
    setActive(key);
    setOpenNote(null);
  };

  const activeCategory = active.startsWith('cat-')
    ? Number(active.slice(4))
    : undefined;

  return (
    <div className="h-screen flex">
      <Sidebar sections={sections} active={active} onSelect={select} />
      {active === 'inbox' && (
        <NoteList date={new Date().toISOString().slice(0, 10)} onOpen={setOpenNote} />
      )}
      {activeCategory !== undefined && (
        <NoteList categoryId={activeCategory} onOpen={setOpenNote} />
      )}
      {active === 'review' && <ReviewPage onDone={refresh} />}
      {active === 'kb' && <KnowledgeBase />}
      {active === 'settings' && <SettingsPage onChanged={refresh} />}
      {openNote && active !== 'kb' && (
        <div className="w-1/2 border-l border-gray-200">
          <div className="h-full flex flex-col">
            <div className="flex justify-end p-2">
              <button onClick={() => setOpenNote(null)} className="text-gray-400 text-sm px-2">
                关闭
              </button>
            </div>
            <NoteDetail id={openNote} />
          </div>
        </div>
      )}
    </div>
  );
}
