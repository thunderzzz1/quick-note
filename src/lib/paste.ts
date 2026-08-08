export function extractImageFromClipboardEvent(evt: ClipboardEvent): File | null {
  const items = evt.clipboardData?.items;
  if (!items) return null;
  for (const item of Array.from(items)) {
    if (item.kind === 'file' && item.type.startsWith('image/')) {
      return item.getAsFile();
    }
  }
  return null;
}

export async function fileToPastedImage(file: File) {
  const buf = await file.arrayBuffer();
  return {
    filename: file.name || 'clipboard-image',
    mime: file.type || 'image/png',
    bytes: Array.from(new Uint8Array(buf)),
  };
}

export function imageMarkdown(relPath: string): string {
  return `![图片](${relPath})`;
}
