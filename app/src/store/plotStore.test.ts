import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { usePlotStore } from './plotStore';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('../services/unifiedEventService', () => ({
  unifiedEventService: {
    addEventListener: vi.fn(),
    removeAllListeners: vi.fn(),
  },
  EventCategory: {
    Plot: 'plot',
  },
}));

describe('plotStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with empty plots', () => {
      const store = usePlotStore();

      expect(store.plots.value).toEqual([]);
      expect(store.currentPlot).toBeNull();
      expect(store.plotCount.value).toBe(0);
    });
  });

  describe('loadAllPlots', () => {
    it('should load all plots from backend', async () => {
      const mockPlots = [
        {
          id: 'plot1',
          mime_type: 'image/png',
          data: 'base64data',
          timestamp: Date.now(),
        },
        {
          id: 'plot2',
          mime_type: 'image/jpeg',
          data: 'base64data2',
          timestamp: Date.now() + 1000,
        },
      ];

      (invoke as any).mockResolvedValue(mockPlots);

      const store = usePlotStore();
      await store.loadAllPlots();

      expect(store.plots.value.length).toBe(2);
      expect(store.plotCount.value).toBe(2);
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const store = usePlotStore();
      await store.loadAllPlots();

      // Should not throw, but plots should remain empty
      expect(store.plots.value.length).toBe(0);
    });
  });

  describe('getPlot', () => {
    it('should get a plot by id', async () => {
      const mockPlot = {
        id: 'plot1',
        mime_type: 'image/png',
        data: 'base64data',
        timestamp: Date.now(),
      };

      (invoke as any).mockResolvedValue(mockPlot);

      const store = usePlotStore();
      const plot = await store.getPlot('plot1');

      expect(plot).toEqual(mockPlot);
      expect(invoke).toHaveBeenCalledWith('get_plot', { plotId: 'plot1' });
    });

    it('should return null on error', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const store = usePlotStore();
      const plot = await store.getPlot('plot1');

      expect(plot).toBeNull();
    });
  });

  describe('deletePlot', () => {
    it('should delete a plot', async () => {
      (invoke as any).mockResolvedValue(true);

      const store = usePlotStore();
      // First add a plot
      store.plots.value = [
        {
          id: 'plot1',
          mime_type: 'image/png',
          data: 'base64data',
          timestamp: Date.now(),
        },
      ];

      const deleted = await store.deletePlot('plot1');

      expect(deleted).toBe(true);
      expect(invoke).toHaveBeenCalledWith('delete_plot', { plotId: 'plot1' });
    });

    it('should return false on error', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const store = usePlotStore();
      const deleted = await store.deletePlot('plot1');

      expect(deleted).toBe(false);
    });
  });

  describe('clearAllPlots', () => {
    it('should clear all plots', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const store = usePlotStore();
      store.plots.value = [
        {
          id: 'plot1',
          mime_type: 'image/png',
          data: 'base64data',
          timestamp: Date.now(),
        },
      ];

      await store.clearAllPlots();

      expect(store.plots.value.length).toBe(0);
      expect(invoke).toHaveBeenCalledWith('clear_all_plots');
    });
  });

  describe('setCurrentPlot', () => {
    it('should set current plot', () => {
      const store = usePlotStore();
      store.plots.value = [
        {
          id: 'plot1',
          mime_type: 'image/png',
          data: 'base64data',
          timestamp: Date.now(),
        },
      ];

      store.setCurrentPlot('plot1');

      expect(store.currentPlot.value?.id).toBe('plot1');
    });
  });

  describe('plot navigation', () => {
    it('should get next plot', () => {
      const store = usePlotStore();
      store.plots.value = [
        { id: 'plot1', mime_type: 'image/png', data: '', timestamp: 1 },
        { id: 'plot2', mime_type: 'image/png', data: '', timestamp: 2 },
        { id: 'plot3', mime_type: 'image/png', data: '', timestamp: 3 },
      ];
      store.setCurrentPlot('plot1');

      store.getNextPlot();

      expect(store.currentPlot.value?.id).toBe('plot2');
    });

    it('should wrap to first plot when at end', () => {
      const store = usePlotStore();
      store.plots.value = [
        { id: 'plot1', mime_type: 'image/png', data: '', timestamp: 1 },
        { id: 'plot2', mime_type: 'image/png', data: '', timestamp: 2 },
      ];
      store.setCurrentPlot('plot2');

      store.getNextPlot();

      expect(store.currentPlot.value?.id).toBe('plot1');
    });

    it('should get previous plot', () => {
      const store = usePlotStore();
      store.plots.value = [
        { id: 'plot1', mime_type: 'image/png', data: '', timestamp: 1 },
        { id: 'plot2', mime_type: 'image/png', data: '', timestamp: 2 },
      ];
      store.setCurrentPlot('plot2');

      store.getPreviousPlot();

      expect(store.currentPlot.value?.id).toBe('plot1');
    });

    it('should wrap to last plot when at beginning', () => {
      const store = usePlotStore();
      store.plots.value = [
        { id: 'plot1', mime_type: 'image/png', data: '', timestamp: 1 },
        { id: 'plot2', mime_type: 'image/png', data: '', timestamp: 2 },
      ];
      store.setCurrentPlot('plot1');

      store.getPreviousPlot();

      expect(store.currentPlot.value?.id).toBe('plot2');
    });

    it('should get first plot', () => {
      const store = usePlotStore();
      store.plots.value = [
        { id: 'plot1', mime_type: 'image/png', data: '', timestamp: 1 },
        { id: 'plot2', mime_type: 'image/png', data: '', timestamp: 2 },
      ];

      store.getFirstPlot();

      expect(store.currentPlot.value?.id).toBe('plot1');
    });

    it('should get last plot', () => {
      const store = usePlotStore();
      store.plots.value = [
        { id: 'plot1', mime_type: 'image/png', data: '', timestamp: 1 },
        { id: 'plot2', mime_type: 'image/png', data: '', timestamp: 2 },
      ];

      store.getLastPlot();

      expect(store.currentPlot.value?.id).toBe('plot2');
    });
  });

  describe('setPlotServerPort', () => {
    it('should set plot server port', () => {
      const store = usePlotStore();
      store.setPlotServerPort(8080);

      expect(store.plotServerPort()).toBe(8080);
    });
  });
});


