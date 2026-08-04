import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { PluginSandbox } from '../plugin-sandbox';
import type { PetState } from '../../types/pet';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    clear: () => { store = {}; },
  };
})();
Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock, writable: true });

describe('PluginSandbox', () => {
  let onSetState: ReturnType<typeof vi.fn>;
  let onNotify: ReturnType<typeof vi.fn>;
  let sandbox: PluginSandbox;

  beforeEach(() => {
    onSetState = vi.fn();
    onNotify = vi.fn();
    sandbox = new PluginSandbox(
      onSetState as (state: PetState) => void,
      onNotify as (text: string) => void,
    );
    localStorageMock.clear();
  });

  afterEach(() => {
    sandbox.destroy();
    vi.useRealTimers();
  });

  it('api.setState calls onSetState callback', () => {
    sandbox.api.setState('working');
    expect(onSetState).toHaveBeenCalledWith('working');
  });

  it('api.notify calls onNotify callback', () => {
    sandbox.api.notify('Hello!');
    expect(onNotify).toHaveBeenCalledWith('Hello!');
  });

  it('api.onEvent + dispatch routes to handler', () => {
    const handler = vi.fn();
    sandbox.api.onEvent('task:start', handler);
    sandbox.dispatch('task:start');
    expect(handler).toHaveBeenCalledOnce();
  });

  it('dispatch does not call handlers for different events', () => {
    const handler = vi.fn();
    sandbox.api.onEvent('task:start', handler);
    sandbox.dispatch('task:done');
    expect(handler).not.toHaveBeenCalled();
  });

  it('loadPlugin executes code with pet API', () => {
    sandbox.loadPlugin(`pet.setState('waving');`);
    expect(onSetState).toHaveBeenCalledWith('waving');
  });

  it('loadPlugin registers onEvent handler via code', () => {
    sandbox.loadPlugin(`pet.onEvent('task:start', () => pet.notify('started'));`);
    sandbox.dispatch('task:start');
    expect(onNotify).toHaveBeenCalledWith('started');
  });

  it('storage.set and storage.get round-trip', () => {
    sandbox.api.storage.set('myKey', { count: 42 });
    const value = sandbox.api.storage.get('myKey') as { count: number };
    expect(value.count).toBe(42);
  });

  it('storage.get returns null for missing key', () => {
    expect(sandbox.api.storage.get('missing')).toBeNull();
  });

  it('destroy clears event handlers', () => {
    const handler = vi.fn();
    sandbox.api.onEvent('task:start', handler);
    sandbox.destroy();
    sandbox.dispatch('task:start');
    expect(handler).not.toHaveBeenCalled();
  });

  it('api.onTick fires on interval (min 1000ms)', () => {
    vi.useFakeTimers();
    const ticker = vi.fn();
    sandbox.api.onTick(500, ticker); // 500ms → clamped to 1000ms
    vi.advanceTimersByTime(1000);
    expect(ticker).toHaveBeenCalledOnce();
    vi.advanceTimersByTime(1000);
    expect(ticker).toHaveBeenCalledTimes(2);
  });
});
