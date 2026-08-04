import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import SpeechBubble from '../SpeechBubble';

describe('SpeechBubble', () => {
  afterEach(() => vi.useRealTimers());

  it('renders text when text prop is provided', () => {
    render(<SpeechBubble text="Hello!" onHide={vi.fn()} />);
    expect(screen.getByText('Hello!')).toBeTruthy();
  });

  it('returns null when text is null', () => {
    const { container } = render(<SpeechBubble text={null} onHide={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('calls onHide after 3 seconds', () => {
    vi.useFakeTimers();
    const onHide = vi.fn();
    render(<SpeechBubble text="Hello!" onHide={onHide} />);

    expect(onHide).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(3000);
    });

    expect(onHide).toHaveBeenCalledOnce();
  });

  it('resets timer when text changes', () => {
    vi.useFakeTimers();
    const onHide = vi.fn();
    const { rerender } = render(<SpeechBubble text="First" onHide={onHide} />);

    act(() => { vi.advanceTimersByTime(2000); });
    expect(onHide).not.toHaveBeenCalled();

    rerender(<SpeechBubble text="Second" onHide={onHide} />);

    act(() => { vi.advanceTimersByTime(2000); }); // only 2s after rerender
    expect(onHide).not.toHaveBeenCalled();

    act(() => { vi.advanceTimersByTime(1000); }); // now 3s after rerender
    expect(onHide).toHaveBeenCalledOnce();
  });
});
