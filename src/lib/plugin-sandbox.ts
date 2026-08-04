import type { PetState } from '../types/pet';

export interface PetAPI {
  setState: (state: PetState) => void;
  notify: (text: string) => void;
  onEvent: (event: string, fn: () => void) => void;
  onTick: (ms: number, fn: () => void) => void;
  schedule: (time: string, fn: () => void) => void;
  storage: {
    get: (key: string) => unknown;
    set: (key: string, value: unknown) => void;
  };
}

export class PluginSandbox {
  private eventHandlers = new Map<string, Array<() => void>>();
  private intervals: ReturnType<typeof setInterval>[] = [];

  constructor(
    private onSetState: (state: PetState) => void,
    private onNotify: (text: string) => void,
    private storagePrefix = 'pet-plugin-',
  ) {}

  get api(): PetAPI {
    return {
      setState: (state) => this.onSetState(state),
      notify: (text) => this.onNotify(text),
      onEvent: (event, fn) => this.addEventHandler(event, fn),
      onTick: (ms, fn) => {
        const interval = setInterval(fn, Math.max(1000, ms));
        this.intervals.push(interval);
      },
      schedule: (time, fn) => this.scheduleAt(time, fn),
      storage: {
        get: (key) => {
          try {
            const raw = localStorage.getItem(`${this.storagePrefix}${key}`);
            return raw !== null ? JSON.parse(raw) : null;
          } catch {
            return null;
          }
        },
        set: (key, value) => {
          localStorage.setItem(`${this.storagePrefix}${key}`, JSON.stringify(value));
        },
      },
    };
  }

  private addEventHandler(event: string, fn: () => void): void {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, []);
    }
    this.eventHandlers.get(event)!.push(fn);
  }

  private scheduleAt(time: string, fn: () => void): void {
    const [h, m] = time.split(':').map(Number);
    const check = () => {
      const now = new Date();
      if (now.getHours() === h && now.getMinutes() === m) {
        fn();
      }
    };
    const interval = setInterval(check, 60_000);
    this.intervals.push(interval);
  }

  loadPlugin(code: string): void {
    try {
      // eslint-disable-next-line no-new-func
      const fn = new Function('pet', code);
      fn(this.api);
    } catch (err) {
      console.error('[plugin-sandbox] Plugin error:', err);
    }
  }

  dispatch(eventType: string): void {
    const handlers = this.eventHandlers.get(eventType) ?? [];
    handlers.forEach((fn) => {
      try { fn(); } catch (err) { console.error('[plugin-sandbox] Handler error:', err); }
    });
  }

  destroy(): void {
    this.intervals.forEach(clearInterval);
    this.intervals = [];
    this.eventHandlers.clear();
  }
}
