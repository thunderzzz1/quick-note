import { describe, expect, it } from 'vitest';
import { extractImageFromClipboardEvent, imageMarkdown } from '../lib/paste';

describe('extractImageFromClipboardEvent', () => {
  it('returns null when clipboard has no image', () => {
    const evt = { clipboardData: { items: [] } } as unknown as ClipboardEvent;
    expect(extractImageFromClipboardEvent(evt)).toBeNull();
  });

  it('returns file for image item', () => {
    const file = new File(['x'], 'a.png', { type: 'image/png' });
    const evt = {
      clipboardData: {
        items: [{ kind: 'file', type: 'image/png', getAsFile: () => file }],
      },
    } as unknown as ClipboardEvent;
    const got = extractImageFromClipboardEvent(evt);
    expect(got?.name).toBe('a.png');
    expect(got?.type).toBe('image/png');
  });

  it('builds image markdown reference', () => {
    expect(imageMarkdown('attachments/2026/08/a/img_001.png')).toBe(
      '![图片](attachments/2026/08/a/img_001.png)',
    );
  });
});
