import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SuggestionCard } from '../components/review/SuggestionCard';
import type { Suggestion } from '../types';

const sug: Suggestion = {
  id: 1,
  note_id: '20260808-153012-ab12',
  ai_category: '待办',
  new_category_proposal: null,
  summary: '明天交周报',
  keywords: '["周报"]',
  status: 'suggested',
  created_at: '2026-08-08T20:12:00+08:00',
};

describe('SuggestionCard', () => {
  it('renders summary and actions', () => {
    render(
      <SuggestionCard suggestion={sug} original="明天下午3点交周报" onAccept={() => {}} onSkip={() => {}} />,
    );
    expect(screen.getByText('明天交周报')).toBeTruthy();
    expect(screen.getByText('✓ 接受')).toBeTruthy();
  });

  it('accept triggers callback', () => {
    const onAccept = vi.fn();
    render(<SuggestionCard suggestion={sug} original="" onAccept={onAccept} onSkip={() => {}} />);
    screen.getByText('✓ 接受').click();
    expect(onAccept).toHaveBeenCalledWith(sug);
  });

  it('shows new category proposal', () => {
    render(
      <SuggestionCard
        suggestion={{ ...sug, new_category_proposal: '购物清单' }}
        original=""
        onAccept={() => {}}
        onSkip={() => {}}
      />,
    );
    expect(screen.getByText('🆕 建议新分类「购物清单」')).toBeTruthy();
  });
});
