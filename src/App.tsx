import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { CaptureWindow } from './components/capture/CaptureWindow';
import { MainWindow } from './components/main/MainWindow';
import { FirstRunWizard } from './components/onboarding/FirstRunWizard';
import { api } from './lib/tauri';

export default function App() {
  const [label, setLabel] = useState<string | null>(null);
  const [configured, setConfigured] = useState<boolean | null>(null);

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

  useEffect(() => {
    if (label === 'main') {
      void api.getSettings().then((s) => setConfigured(s.configured));
    }
  }, [label]);

  if (label === null) return null;
  if (label === 'capture') return <CaptureWindow />;
  if (configured === false) return <FirstRunWizard onDone={() => setConfigured(true)} />;
  return <MainWindow />;
}
