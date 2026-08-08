import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Sidebar, type SidebarSection } from '../components/main/Sidebar';

const sections: SidebarSection[] = [
  { key: 'inbox', label: '今日速记', icon: '📥' },
  { key: 'review', label: '整理建议', icon: '✨', badge: 3 },
  { key: 'kb', label: '知识库', icon: '📚' },
];

describe('Sidebar', () => {
  it('renders sections and badge', () => {
    render(<Sidebar sections={sections} active="inbox" onSelect={() => {}} />);
    expect(screen.getByText('今日速记')).toBeTruthy();
    expect(screen.getByText('3')).toBeTruthy();
  });

  it('calls onSelect with key', () => {
    const onSelect = vi.fn();
    render(<Sidebar sections={sections} active="inbox" onSelect={onSelect} />);
    screen.getByText('知识库').click();
    expect(onSelect).toHaveBeenCalledWith('kb');
  });
});
