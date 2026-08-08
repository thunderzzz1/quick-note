import { useEffect, useRef, useState } from 'react';

export function useAutosave(
  getContent: () => string,
  onSave: (md: string) => Promise<void>,
  delay = 800,
) {
  const timer = useRef<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [lastSavedAt, setLastSavedAt] = useState<number | null>(null);

  useEffect(() => {
    return () => {
      if (timer.current) window.clearTimeout(timer.current);
    };
  }, []);

  const flush = async () => {
    if (timer.current) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    const md = getContent();
    if (!md.trim()) return;
    setSaving(true);
    try {
      await onSave(md);
      setLastSavedAt(Date.now());
    } finally {
      setSaving(false);
    }
  };

  const schedule = () => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void flush(), delay);
  };

  return { saving, lastSavedAt, schedule, flush };
}
