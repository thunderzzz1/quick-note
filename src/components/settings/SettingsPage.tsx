import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, type SettingsDto } from '../../lib/tauri';

export function SettingsPage({ onChanged }: { onChanged?: () => void }) {
  const [s, setS] = useState<SettingsDto | null>(null);
  const [msg, setMsg] = useState('');

  useEffect(() => {
    void api.getSettings().then(setS);
  }, []);

  if (!s) return <div className="flex-1 p-6 text-gray-400">加载中…</div>;

  const save = async () => {
    await api.updateSettings(s);
    setMsg('已保存');
    setTimeout(() => setMsg(''), 2000);
    onChanged?.();
  };

  const changeDir = async () => {
    const dir = await open({ directory: true });
    if (typeof dir === 'string') {
      const dataDir = await api.migrateDataDir(dir);
      setS({ ...s, data_dir: dataDir });
      setMsg('数据目录已迁移');
      setTimeout(() => setMsg(''), 2000);
      onChanged?.();
    }
  };

  const input =
    'w-full border border-gray-200 rounded-md px-3 py-1.5 text-sm mb-3';
  const label = 'block text-sm text-gray-600 mb-1';

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-xl">
      <h2 className="text-base font-bold mb-4">设置</h2>
      <label className={label}>数据目录</label>
      <div className="flex gap-2 mb-3">
        <input className={`${input} mb-0 flex-1 bg-gray-50`} value={s.data_dir} readOnly />
        <button
          onClick={() => void changeDir()}
          className="bg-gray-100 text-gray-700 rounded px-3 py-1.5 text-sm shrink-0"
        >
          更改…
        </button>
      </div>
      <label className={label}>全局快捷键</label>
      <input
        className={input}
        value={s.hotkey}
        onChange={(e) => setS({ ...s, hotkey: e.target.value })}
      />
      <label className={label}>每日整理时间（HH:MM）</label>
      <input
        className={input}
        value={s.org_time}
        onChange={(e) => setS({ ...s, org_time: e.target.value })}
      />
      <label className={label}>API Base URL</label>
      <input
        className={input}
        value={s.ai_base_url}
        onChange={(e) => setS({ ...s, ai_base_url: e.target.value })}
      />
      <label className={label}>模型</label>
      <input
        className={input}
        value={s.ai_model}
        onChange={(e) => setS({ ...s, ai_model: e.target.value })}
      />
      <label className={label}>API Key</label>
      <input
        type="password"
        className={input}
        value={s.ai_api_key}
        onChange={(e) => setS({ ...s, ai_api_key: e.target.value })}
      />
      <label className="flex items-center gap-2 text-sm text-gray-600 mb-4">
        <input
          type="checkbox"
          checked={s.auto_org_enabled}
          onChange={(e) => setS({ ...s, auto_org_enabled: e.target.checked })}
        />
        每晚自动整理
      </label>
      <button onClick={() => void save()} className="bg-blue-500 text-white rounded px-4 py-1.5 text-sm">
        保存
      </button>
      {msg ? <span className="ml-3 text-sm text-green-600">{msg}</span> : null}
    </div>
  );
}
