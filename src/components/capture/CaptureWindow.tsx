import { useEffect, useRef, useState } from 'react';
import { Crepe } from '@milkdown/crepe';
import { editorViewCtx, parserCtx } from '@milkdown/kit/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAutosave } from '../../lib/autosave';
import {
  extractImageFromClipboardEvent,
  fileToPastedImage,
  imageMarkdown,
} from '../../lib/paste';
import { api } from '../../lib/tauri';
import { TodayStrip } from './TodayStrip';

function setEditorMarkdown(crepe: Crepe, md: string): void {
  crepe.editor.action((ctx) => {
    const parser = ctx.get(parserCtx);
    const view = ctx.get(editorViewCtx);
    const doc = parser(md);
    view.dispatch(
      view.state.tr.replaceWith(0, view.state.doc.content.size, doc.content),
    );
  });
}

export function CaptureWindow() {
  const hostRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<Crepe | null>(null);
  const currentId = useRef<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (!hostRef.current || editorRef.current) return;
    const crepe = new Crepe({
      root: hostRef.current,
      defaultValue: '',
    });
    editorRef.current = crepe;
    void crepe.create().then(() => {
      crepe.editor.action((ctx) => ctx.get(editorViewCtx).focus());
    });
  }, []);

  useEffect(() => {
    return () => {
      void editorRef.current?.destroy();
      editorRef.current = null;
    };
  }, []);

  useEffect(() => {
    const unlisten = listen('capture-focus', () => {
      const crepe = editorRef.current;
      if (crepe) {
        crepe.editor.action((ctx) => ctx.get(editorViewCtx).focus());
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);

  const doSave = async (md: string) => {
    if (currentId.current) {
      await api.updateNote(currentId.current, md);
    } else {
      const result = await api.saveNote(md, []);
      currentId.current = result.id;
    }
    setRefreshKey((k) => k + 1);
  };

  const { saving, lastSavedAt, flush } = useAutosave(
    () => editorRef.current?.getMarkdown() ?? '',
    doSave,
  );
  const flushRef = useRef(flush);
  flushRef.current = flush;

  useEffect(() => {
    const onPaste = async (evt: ClipboardEvent) => {
      const file = extractImageFromClipboardEvent(evt);
      if (!file) return;
      evt.preventDefault();
      const img = await fileToPastedImage(file);
      const md = editorRef.current?.getMarkdown() ?? '';
      let rel: string;
      if (currentId.current) {
        rel = await api.saveImage(currentId.current, img);
        await api.updateNote(currentId.current, md);
      } else {
        const result = await api.saveNote(md, [img]);
        currentId.current = result.id;
        rel = result.image_refs[0];
      }
      setEditorMarkdown(editorRef.current!, `${md}\n${imageMarkdown(rel)}\n`);
      setRefreshKey((k) => k + 1);
    };
    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        void flushRef.current();
        return;
      }
      if (e.key === 'Escape') {
        void flushRef.current();
        void getCurrentWindow().hide();
      }
    };
    const onBlur = () => {
      void flushRef.current();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('blur', onBlur);
    };
  }, []);

  return (
    <div className="h-screen flex flex-col bg-white">
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-gray-100">
        <span className="text-xs text-gray-400">
          {saving
            ? '保存中…'
            : lastSavedAt
              ? `已保存 ${new Date(lastSavedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
              : '未保存'}
        </span>
        <button
          onClick={() => void flush()}
          className="ml-auto bg-blue-500 text-white text-xs rounded px-3 py-1"
        >
          保存
        </button>
      </div>
      <div ref={hostRef} className="flex-1 overflow-y-auto px-3 py-2" />
      <TodayStrip refreshKey={refreshKey} />
    </div>
  );
}
