import { describe, it, expect, beforeEach } from 'vitest';
import { createEventBus, appEventBus, type AppEvents } from './eventBus';

describe('eventBus', () => {
  describe('createEventBus', () => {
    it('should create an event bus instance', () => {
      const bus = createEventBus<{ test: string }>();
      expect(bus).toBeDefined();
      expect(bus.on).toBeDefined();
      expect(bus.off).toBeDefined();
      expect(bus.emit).toBeDefined();
    });

    it('should register and call event handlers', () => {
      const bus = createEventBus<{ test: string }>();
      let receivedPayload = '';

      bus.on('test', (payload) => {
        receivedPayload = payload;
      });

      bus.emit('test', 'hello');
      expect(receivedPayload).toBe('hello');
    });

    it('should support multiple handlers for the same event', () => {
      const bus = createEventBus<{ test: string }>();
      const calls: string[] = [];

      bus.on('test', (payload) => {
        calls.push(`handler1: ${payload}`);
      });

      bus.on('test', (payload) => {
        calls.push(`handler2: ${payload}`);
      });

      bus.emit('test', 'hello');
      expect(calls).toHaveLength(2);
      expect(calls).toContain('handler1: hello');
      expect(calls).toContain('handler2: hello');
    });

    it('should unregister handlers', () => {
      const bus = createEventBus<{ test: string }>();
      let callCount = 0;

      const handler = () => {
        callCount++;
      };

      bus.on('test', handler);
      bus.emit('test', 'hello');
      expect(callCount).toBe(1);

      bus.off('test', handler);
      bus.emit('test', 'hello');
      expect(callCount).toBe(1); // Should not increment
    });

    it('should return an unsubscribe function from on', () => {
      const bus = createEventBus<{ test: string }>();
      let callCount = 0;

      const handler = () => {
        callCount++;
      };

      const unsubscribe = bus.on('test', handler);
      bus.emit('test', 'hello');
      expect(callCount).toBe(1);

      unsubscribe();
      bus.emit('test', 'hello');
      expect(callCount).toBe(1); // Should not increment
    });

    it('should handle events with different payload types', () => {
      type Events = {
        string: string;
        number: number;
        object: { key: string };
      };

      const bus = createEventBus<Events>();

      let stringValue = '';
      let numberValue = 0;
      let objectValue: { key: string } | null = null;

      bus.on('string', (payload) => {
        stringValue = payload;
      });

      bus.on('number', (payload) => {
        numberValue = payload;
      });

      bus.on('object', (payload) => {
        objectValue = payload;
      });

      bus.emit('string', 'test');
      bus.emit('number', 42);
      bus.emit('object', { key: 'value' });

      expect(stringValue).toBe('test');
      expect(numberValue).toBe(42);
      expect(objectValue).toEqual({ key: 'value' });
    });

    it('should not throw when emitting to non-existent event', () => {
      const bus = createEventBus<{ test: string }>();
      expect(() => {
        bus.emit('nonexistent' as any, 'hello');
      }).not.toThrow();
    });

    it('should not throw when unregistering non-existent handler', () => {
      const bus = createEventBus<{ test: string }>();
      expect(() => {
        bus.off('test', () => {});
      }).not.toThrow();
    });
  });

  describe('appEventBus', () => {
    beforeEach(() => {
      // Clear all listeners before each test
      // Note: The event bus doesn't have a clear method, so we'll work with what we have
    });

    it('should be a valid event bus instance', () => {
      expect(appEventBus).toBeDefined();
      expect(appEventBus.on).toBeDefined();
      expect(appEventBus.off).toBeDefined();
      expect(appEventBus.emit).toBeDefined();
    });

    it('should handle plot navigator update events', () => {
      let receivedPayload: string[] | null = null;

      const unsubscribe = appEventBus.on('plot:navigator-update', (payload) => {
        receivedPayload = payload;
      });

      appEventBus.emit('plot:navigator-update', ['plot1', 'plot2']);
      expect(receivedPayload).toEqual(['plot1', 'plot2']);

      unsubscribe();
    });

    it('should handle orchestrator project change status events', () => {
      let receivedPayload: { message: string; progress_percentage: number } | null = null;

      const unsubscribe = appEventBus.on('orchestrator:project-change-status', (payload) => {
        receivedPayload = payload;
      });

      appEventBus.emit('orchestrator:project-change-status', {
        message: 'Loading...',
        progress_percentage: 50,
      });

      expect(receivedPayload).toEqual({
        message: 'Loading...',
        progress_percentage: 50,
      });

      unsubscribe();
    });

    it('should handle orchestrator selected directory events', () => {
      let receivedPayload: { path: string; is_julia_project: boolean } | null = null;

      const unsubscribe = appEventBus.on('orchestrator:selected-directory', (payload) => {
        receivedPayload = payload;
      });

      appEventBus.emit('orchestrator:selected-directory', {
        path: '/path/to/project',
        is_julia_project: true,
      });

      expect(receivedPayload).toEqual({
        path: '/path/to/project',
        is_julia_project: true,
      });

      unsubscribe();
    });

    it('should handle LSP status events', () => {
      let receivedPayload: {
        status: string;
        message?: string;
        error?: string | null;
        project_path?: string | null;
      } | null = null;

      const unsubscribe = appEventBus.on('lsp:status', (payload) => {
        receivedPayload = payload;
      });

      appEventBus.emit('lsp:status', {
        status: 'ready',
        message: 'LSP is ready',
        error: null,
        project_path: '/path/to/project',
      });

      expect(receivedPayload).toEqual({
        status: 'ready',
        message: 'LSP is ready',
        error: null,
        project_path: '/path/to/project',
      });

      unsubscribe();
    });
  });
});


