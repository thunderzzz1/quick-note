import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { CaptureWindow } from './components/capture/CaptureWindow';

export default function App() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const w = await getCurrentWindow();
        if (!cancelled) setLabel(w.label);
      } catch {
        if (!cancelled) setLabel('main');
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (label === null) return null;
  if (label === 'capture') return <CaptureWindow />;
  return (
    <div className="h-screen grid place-items-center text-gray-400">
      主界面（下一步实现）
    </div>
  );
}
