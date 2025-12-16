// Lightweight typed event bus for Vue/TS

export type EventHandler<Payload> = (payload: Payload) => void;

export interface TypedEventBus<Events extends Record<string, unknown>> {
  on<K extends keyof Events>(event: K, handler: EventHandler<Events[K]>): () => void;
  off<K extends keyof Events>(event: K, handler: EventHandler<Events[K]>): void;
  emit<K extends keyof Events>(event: K, payload: Events[K]): void;
}

export function createEventBus<Events extends Record<string, unknown>>(): TypedEventBus<Events> {
  const listeners = new Map<string, Set<Function>>();

  function on<K extends keyof Events>(event: K, handler: EventHandler<Events[K]>): () => void {
    const key = String(event);
    let set = listeners.get(key);
    if (!set) {
      set = new Set();
      listeners.set(key, set);
    }
    set.add(handler as unknown as Function);
    return () => off(event, handler);
  }

  function off<K extends keyof Events>(event: K, handler: EventHandler<Events[K]>) {
    const key = String(event);
    const set = listeners.get(key);
    if (!set) return;
    set.delete(handler as unknown as Function);
    if (set.size === 0) listeners.delete(key);
  }

  function emit<K extends keyof Events>(event: K, payload: Events[K]) {
    const key = String(event);
    const set = listeners.get(key);
    if (!set) return;
    for (const fn of set) (fn as EventHandler<Events[K]>)(payload);
  }

  return { on, off, emit };
}

// Example event map (extend in features as needed)
export type AppEvents = {
  'plot:navigator-update': string[];
  'orchestrator:project-change-status': { message: string; progress_percentage: number };
  'orchestrator:selected-directory': { path: string; is_julia_project: boolean };
  'lsp:status': {
    status: string;
    message?: string;
    error?: string | null;
    project_path?: string | null;
  };
};

// Optional shared bus instance (features may also create scoped buses)
export const appEventBus = createEventBus<AppEvents>();
