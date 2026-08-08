import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { api } from '../../lib/tauri';

export function FirstRunWizard({ onDone }: { onDone: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const choose = async () => {
    setError('');
    const dir = await open({ directory: true });
    if (typeof dir !== 'string') return;
    setBusy(true);
    try {
      await api.initDataDir(dir);
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="h-screen grid place-items-center bg-gray-50">
      <div className="w-96 bg-white rounded-xl shadow p-8 text-center">
        <div className="text-2xl font-bold text-gray-800 mb-2">QuickNote</div>
        <p className="text-sm text-gray-500 mb-6">
          欢迎使用！先选择数据存放目录（推荐放在非系统盘，例如 D 盘）。
          <br />
          所有笔记、图片和配置都会保存在这里。
        </p>
        <button
          onClick={() => void choose()}
          disabled={busy}
          className="bg-blue-500 text-white rounded-lg px-6 py-2 text-sm disabled:opacity-50"
        >
          {busy ? '初始化中…' : '选择数据目录'}
        </button>
        {error ? <p className="text-xs text-red-500 mt-3">{error}</p> : null}
      </div>
    </div>
  );
}
