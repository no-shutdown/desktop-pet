import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ContextMenu from '../ContextMenu';

describe('ContextMenu', () => {
  const defaultProps = {
    x: 100, y: 100,
    onClose: vi.fn(),
    onSwitchPet: vi.fn(),
    onOpenCreator: vi.fn(),
    onExit: vi.fn(),
  };

  it('renders all menu items when visible', () => {
    render(<ContextMenu {...defaultProps} visible={true} />);
    expect(screen.getByText('Switch Pet')).toBeTruthy();
    expect(screen.getByText('Open Creator')).toBeTruthy();
    expect(screen.getByText('Exit')).toBeTruthy();
  });

  it('renders nothing when not visible', () => {
    render(<ContextMenu {...defaultProps} visible={false} />);
    expect(screen.queryByText('Open Creator')).toBeNull();
  });

  it('calls onSwitchPet when Switch Pet clicked', () => {
    const onSwitchPet = vi.fn();
    render(<ContextMenu {...defaultProps} visible={true} onSwitchPet={onSwitchPet} />);
    fireEvent.click(screen.getByText('Switch Pet'));
    expect(onSwitchPet).toHaveBeenCalledOnce();
  });

  it('calls onOpenCreator when clicked', () => {
    const onOpenCreator = vi.fn();
    render(<ContextMenu {...defaultProps} visible={true} onOpenCreator={onOpenCreator} />);
    fireEvent.click(screen.getByText('Open Creator'));
    expect(onOpenCreator).toHaveBeenCalledOnce();
  });

  it('calls onExit when clicked', () => {
    const onExit = vi.fn();
    render(<ContextMenu {...defaultProps} visible={true} onExit={onExit} />);
    fireEvent.click(screen.getByText('Exit'));
    expect(onExit).toHaveBeenCalledOnce();
  });

  it('calls onClose when backdrop clicked', () => {
    const onClose = vi.fn();
    render(<ContextMenu {...defaultProps} visible={true} onClose={onClose} />);
    fireEvent.click(document.querySelector('[data-testid="backdrop"]')!);
    expect(onClose).toHaveBeenCalledOnce();
  });
});
