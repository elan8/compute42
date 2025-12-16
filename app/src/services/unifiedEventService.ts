import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { debug, trace, logError } from '../utils/logger';

// Unified event structure from internals library
export interface UnifiedEvent {
  category: string;
  event_type: string;
  payload: any;
  timestamp?: number;
}

// Event categories
export enum EventCategory {
  Account = 'account',
  Orchestrator = 'orchestrator',
  Lsp = 'lsp',
  Julia = 'julia',
  Plot = 'plot',
  File = 'file',
  Communication = 'communication',
  System = 'system',
}

// Event handler type
export type EventHandler = (event: UnifiedEvent) => void | Promise<void>;

// Event listener registration
export interface EventListenerRegistration {
  category: EventCategory;
  eventType: string;
  handlers: Set<EventHandler>;
  unlistenFn?: UnlistenFn;
}

export class UnifiedEventService {
  private listeners: Map<string, EventListenerRegistration> = new Map();
  private isInitialized = false;

  constructor() {
    this.initialize();
  }

  private async initialize() {
    if (this.isInitialized) return;

    try {
      await trace('UnifiedEventService: Initializing unified event service');
      this.isInitialized = true;
    } catch (error) {
      await logError('UnifiedEventService: Failed to initialize', error);
    }
  }

  /**
   * Register a listener for a specific event category and type
   */
  async addEventListener(
    category: EventCategory,
    eventType: string,
    handler: EventHandler
  ): Promise<void> {
    const eventKey = `${category}:${eventType}`;

    const existing = this.listeners.get(eventKey);
    if (existing) {
      existing.handlers.add(handler);
      return;
    }

    try {
      const handlers = new Set<EventHandler>([handler]);

      const unlistenFn = await listen<any>(eventKey, async (event) => {
        // Reconstruct full UnifiedEvent
        const unifiedEvent: UnifiedEvent = {
          category: eventKey.split(':')[0],
          event_type: eventKey,
          payload: event.payload,
          timestamp: Date.now(),
        };
        // Dispatch to all registered handlers for this eventKey
        const registration = this.listeners.get(eventKey);
        if (!registration) return;
        for (const h of registration.handlers) {
          try {
            await h(unifiedEvent);
          } catch (error) {
            await logError(`UnifiedEventService: Error in handler for ${eventKey}`, error);
          }
        }
      });

      this.listeners.set(eventKey, {
        category,
        eventType,
        handlers,
        unlistenFn,
      });
    } catch (error) {
      await logError(`UnifiedEventService: Failed to register listener for ${eventKey}`, error);
    }
  }

  /**
   * Remove a specific event listener
   */
  async removeEventListener(category: EventCategory, eventType: string): Promise<void> {
    const eventKey = `${category}:${eventType}`;
    const listener = this.listeners.get(eventKey);
    if (!listener) return;
    // Remove all handlers and underlying Tauri listener
    if (listener.unlistenFn) {
      try {
        await listener.unlistenFn();
      } catch (error) {
        await logError(`UnifiedEventService: Failed to remove listener for ${eventKey}`, error);
      }
    }
    this.listeners.delete(eventKey);
    await trace(`UnifiedEventService: Removed listener for ${eventKey}`);
  }

  /**
   * Remove all event listeners
   */
  async removeAllListeners(): Promise<void> {
    const unlistenPromises = Array.from(this.listeners.values()).map(async (listener) => {
      if (listener.unlistenFn) {
        try {
          await listener.unlistenFn();
        } catch (error) {
          await logError(
            `UnifiedEventService: Failed to remove listener for ${listener.category}:${listener.eventType}`,
            error
          );
        }
      }
    });

    await Promise.all(unlistenPromises);
    this.listeners.clear();
    await trace('UnifiedEventService: Removed all listeners');
  }

  /**
   * Get all registered listeners
   */
  getRegisteredListeners(): string[] {
    return Array.from(this.listeners.keys());
  }

  /**
   * Check if a listener is registered
   */
  hasListener(category: EventCategory, eventType: string): boolean {
    const eventKey = `${category}:${eventType}`;
    return this.listeners.has(eventKey);
  }
}

// Export singleton instance
export const unifiedEventService = new UnifiedEventService();
