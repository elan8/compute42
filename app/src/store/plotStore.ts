import { defineStore } from 'pinia';
import { ref, reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { debug, logError, logObject } from '../utils/logger';
import { unifiedEventService, EventCategory } from '../services/unifiedEventService';
import type { PlotData as GenPlotData } from '../types/bindings/shared/PlotData';

export interface PlotData extends GenPlotData {
  // frontend-only derived fields
  image_url?: string;
  imageSrc?: string;
}

export interface PlotThumbnail {
  plot_id: string;
  thumbnail_data: string; // Base64 encoded thumbnail
  timestamp: number;
}

export interface PlotEvent {
  event_type:
    | 'PlotCreated'
    | 'PlotUpdated'
    | 'PlotDeleted'
    | 'ThumbnailGenerated'
    | 'PlotNavigatorUpdate';
  plot_data?: PlotData;
  thumbnail?: PlotThumbnail;
  plot_id?: string;
}

interface PlotState {
  plots: Map<string, PlotData>;
  currentPlotId: string | null;
  isListening: boolean;
  eventUnlistenFn: (() => void) | null;
  plotServerPort: number | null;
}

export const usePlotStore = defineStore('plot', () => {
  const state = reactive<PlotState>({
    plots: new Map(),
    currentPlotId: null,
    isListening: false,
    eventUnlistenFn: null,
    plotServerPort: null,
  });

  // Getters
  const plots = ref<PlotData[]>([]);
  const currentPlot = ref<PlotData | null>(null);
  const plotCount = ref(0);

  // Initialize plot listening immediately when store is created
  debug('PlotStore: Creating plot store and initializing...');

  // Initialize plot listening synchronously
  const initializePlotListening = async () => {
    if (state.isListening) {
      debug('Plot listening already initialized');
      return;
    }

    try {
      debug('Setting up unified plot event listeners...');

      // Set up unified event listener for plot events (names and payloads align with backend)
      await unifiedEventService.addEventListener(
        EventCategory.Plot,
        'plot-added',
        async (event) => {
          logObject('info', 'Received unified plot-added event:', event);
          if (event.payload.plot_data) {
            // Use the plot data directly from the event
            handlePlotEvent({ event_type: 'PlotCreated', plot_data: event.payload.plot_data });
          }
        }
      );

      await unifiedEventService.addEventListener(
        EventCategory.Plot,
        'plot-updated',
        async (event) => {
          logObject('info', 'Received unified plot-updated event:', event);
          if (event.payload.plot_id) {
            const plot = await getPlot(event.payload.plot_id);
            if (plot) {
              handlePlotEvent({ event_type: 'PlotUpdated', plot_data: plot });
            }
          }
        }
      );

      await unifiedEventService.addEventListener(
        EventCategory.Plot,
        'plot-deleted',
        async (event) => {
          logObject('info', 'Received unified plot-deleted event:', event);
          if (event.payload.plot_id) {
            handlePlotEvent({ event_type: 'PlotDeleted', plot_id: event.payload.plot_id });
          }
        }
      );

      // Listen for plot server started/restarted events
      await unifiedEventService.addEventListener(
        EventCategory.Plot,
        'server-started',
        async (event) => {
          logObject('info', 'Received plot server-started event:', event);
          if (event.payload.port) {
            setPlotServerPort(event.payload.port);
            updateAllPlotUrls(event.payload.port);
          }
        }
      );

      await unifiedEventService.addEventListener(
        EventCategory.Plot,
        'server-restarted',
        async (event) => {
          logObject('info', 'Received plot server-restarted event:', event);
          if (event.payload.port) {
            setPlotServerPort(event.payload.port);
            updateAllPlotUrls(event.payload.port);
          }
        }
      );

      // Legacy event listener for backward compatibility
      const unlisten = await listen<PlotEvent>('julia-plot', (event) => {
        logObject('info', 'Received legacy julia-plot event:', event);
        if (
          event.payload &&
          event.payload.event_type === 'PlotCreated' &&
          event.payload.plot_data
        ) {
          handlePlotEvent({ event_type: 'PlotCreated', plot_data: event.payload.plot_data });
        }
      });

      state.eventUnlistenFn = unlisten;
      state.isListening = true;
      debug('Successfully started listening for plot events');
    } catch (err) {
      await logError('Failed to set up plot event listeners', err);
      state.isListening = false;
      throw err;
    }
  };

  // Initialize plot listening only (don't load plots yet)
  initializePlotListening()
    .then(() => {
      debug('PlotStore: Plot listening initialized successfully');
      // Don't load plots here - they will be loaded when the orchestrator is ready
    })
    .catch(async (err) => {
      await logError('PlotStore: Failed to initialize plot listening', err);
    });

  // Update computed values
  const updateComputedValues = () => {
    debug('Updating computed values, current plots in state: ' + state.plots.size);

    // Create a new array to ensure Vue reactivity
    const plotsArray = Array.from(state.plots.values()).sort((a, b) => b.timestamp - a.timestamp);
    plots.value = plotsArray;
    plotCount.value = state.plots.size;

    // Safely update current plot
    if (state.currentPlotId && state.plots.has(state.currentPlotId)) {
      const existingPlot = state.plots.get(state.currentPlotId);
      if (existingPlot) {
        currentPlot.value = existingPlot;
        debug('Set current plot to existing plot: ' + state.currentPlotId);
      }
    } else if (plotsArray.length > 0) {
      currentPlot.value = plotsArray[0];
      state.currentPlotId = plotsArray[0].id;
      debug('Set current plot to first plot: ' + plotsArray[0].id);
    } else {
      currentPlot.value = null;
      state.currentPlotId = null;
      debug('No plots available, cleared current plot');
    }
    debug(
      'Updated computed values - plotCount: ' +
        plotCount.value +
        ' currentPlot: ' +
        (currentPlot.value?.id || 'null')
    );
  };

  // Actions
  const loadPlotImage = async (plotId: string) => {
    try {
      debug('Loading plot image for: ' + plotId);

      // Get the plot from the store
      if (state.plots.has(plotId)) {
        const plot = state.plots.get(plotId)!;

        // Use the image_url from the plot data if available
        if (plot.image_url) {
          plot.imageSrc = plot.image_url;
          updateComputedValues();
          debug('Plot image URL set for: ' + plotId + ' URL: ' + plot.imageSrc);
        } else if (plot.data && plot.mime_type.startsWith('image/')) {
          // Fallback to data URL if no HTTP URL is available
          const dataUrl = `data:${plot.mime_type};base64,${plot.data}`;
          plot.imageSrc = dataUrl;
          updateComputedValues();
          debug('Plot image data URL created for: ' + plotId + ' MIME: ' + plot.mime_type);
        } else {
          debug('No image URL or data available for plot: ' + plotId);
        }
      }
    } catch (err) {
      await logError('Failed to load plot image', err);
    }
  };

  const handlePlotEvent = (event: PlotEvent) => {
    debug('Handling plot event: ' + event.event_type);
    logObject('info', 'Plot event payload:', event);

    switch (event.event_type) {
      case 'PlotCreated':
        if (event.plot_data) {
          debug(
            'Adding plot to store: ' +
              event.plot_data.id +
              ' ' +
              event.plot_data.mime_type +
              ' Data length: ' +
              (event.plot_data.data?.length || 0)
          );
          state.plots.set(event.plot_data.id, event.plot_data);
          updateComputedValues();

          // Load the plot image if it's an image type
          if (event.plot_data.mime_type.startsWith('image/')) {
            debug('Loading image for plot: ' + event.plot_data.id);
            loadPlotImage(event.plot_data.id);
          }

          debug(
            'Plot created and added to store: ' +
              event.plot_data.id +
              ' Total plots: ' +
              state.plots.size
          );
        } else {
          debug('PlotCreated event received but no plot_data provided');
        }
        break;

      case 'PlotUpdated':
        if (event.plot_data) {
          state.plots.set(event.plot_data.id, event.plot_data);
          updateComputedValues();
          debug('Plot updated: ' + event.plot_data.id);
        }
        break;

      case 'PlotDeleted':
        if (event.plot_id) {
          state.plots.delete(event.plot_id);
          if (state.currentPlotId === event.plot_id) {
            state.currentPlotId = null;
          }
          updateComputedValues();
          debug('Plot deleted: ' + event.plot_id);
        }
        break;

      case 'ThumbnailGenerated':
        if (event.thumbnail) {
          debug('Thumbnail generated for plot: ' + event.thumbnail.plot_id);
        }
        break;

      case 'PlotNavigatorUpdate':
        loadAllPlots();
        break;
    }
  };

  const loadAllPlots = async () => {
    try {
      debug('Loading all plots from backend...');
      const allPlots = await invoke<PlotData[]>('get_all_plots');
      debug('Received plots from backend: ' + allPlots.length);
      logObject('info', 'All plots:', allPlots);
      state.plots.clear();
      allPlots.forEach((plot) => {
        state.plots.set(plot.id, plot);
      });
      updateComputedValues();
      debug('Loaded all plots: ' + allPlots.length + ' Total in store: ' + state.plots.size);

      // Load images for all image plots
      for (const plot of allPlots) {
        if (plot.mime_type.startsWith('image/')) {
          loadPlotImage(plot.id);
        }
      }
    } catch (err) {
      await logError('Failed to load plots', err);
    }
  };

  const getPlot = async (plotId: string) => {
    try {
      const plot = await invoke<PlotData | null>('get_plot', { plotId });
      if (plot) {
        state.plots.set(plot.id, plot);
        updateComputedValues();
        return plot;
      }
      return null;
    } catch (error) {
      await logError('Failed to get plot:', error as unknown);
      return null;
    }
  };

  const deletePlot = async (plotId: string) => {
    try {
      const deleted = await invoke<boolean>('delete_plot', { plotId });
      if (deleted) {
        state.plots.delete(plotId);
        if (state.currentPlotId === plotId) {
          state.currentPlotId = null;
        }
        updateComputedValues();
      }
      return deleted;
    } catch (error) {
      await logError('Failed to delete plot:', error as unknown);
      return false;
    }
  };

  const clearAllPlots = async () => {
    try {
      await invoke('clear_all_plots');
      state.plots.clear();
      state.currentPlotId = null;
      updateComputedValues();
    } catch (error) {
      await logError('Failed to clear plots:', error as unknown);
    }
  };

  const setCurrentPlot = (plotId: string) => {
    if (state.plots.has(plotId)) {
      state.currentPlotId = plotId;
      updateComputedValues();
    }
  };

  const getNextPlot = () => {
    if (plots.value.length === 0) return;

    const currentIndex = plots.value.findIndex((plot) => plot.id === state.currentPlotId);
    const nextIndex = (currentIndex + 1) % plots.value.length;
    setCurrentPlot(plots.value[nextIndex].id);
  };

  const getPreviousPlot = () => {
    if (plots.value.length === 0) return;

    const currentIndex = plots.value.findIndex((plot) => plot.id === state.currentPlotId);
    const prevIndex = currentIndex === 0 ? plots.value.length - 1 : currentIndex - 1;
    setCurrentPlot(plots.value[prevIndex].id);
  };

  const getFirstPlot = () => {
    if (plots.value.length > 0) {
      setCurrentPlot(plots.value[0].id);
    }
  };

  const getLastPlot = () => {
    if (plots.value.length > 0) {
      setCurrentPlot(plots.value[plots.value.length - 1].id);
    }
  };

  const stopPlotListening = async () => {
    state.isListening = false;

    if (state.eventUnlistenFn) {
      state.eventUnlistenFn();
      state.eventUnlistenFn = null;
    }
  };

  // Test function to add a sample plot
  const addTestPlot = async () => {
    try {
      await invoke<string>('test_plot_system');
    } catch (error) {
      await logError('Failed to add test plot:', error as unknown);
    }
  };

  // Initialize plots when orchestrator is ready
  const initializePlots = async () => {
    try {
      debug('PlotStore: Initializing plots after orchestrator is ready...');
      await loadAllPlots();
      debug('PlotStore: Plots initialized successfully');
    } catch (error) {
      await logError('PlotStore: Failed to initialize plots', error);
    }
  };

  // Set plot server port
  const setPlotServerPort = (port: number | null) => {
    state.plotServerPort = port;
    debug(`PlotStore: Plot server port set to ${port}`);
  };

  // Update all plot URLs when server port changes
  const updateAllPlotUrls = (newPort: number) => {
    debug(`PlotStore: Updating all plot URLs to use port ${newPort}`);
    let updatedCount = 0;
    for (const [plotId, plot] of state.plots.entries()) {
      if (plot.mime_type.startsWith('image/')) {
        // Update image_url with new port
        plot.image_url = `http://127.0.0.1:${newPort}/plots/${plotId}/image`;
        plot.imageSrc = plot.image_url;
        updatedCount++;
      }
    }
    if (updatedCount > 0) {
      updateComputedValues();
      debug(`PlotStore: Updated ${updatedCount} plot URLs`);
    }
  };

  return {
    // State
    plots,
    currentPlot,
    plotCount,
    isListening: () => state.isListening,

    // Actions
    loadAllPlots,
    getPlot,
    deletePlot,
    clearAllPlots,
    setCurrentPlot,
    getNextPlot,
    getPreviousPlot,
    getFirstPlot,
    getLastPlot,
    stopPlotListening,
    addTestPlot,
    initializePlots,
    setPlotServerPort,
    updateAllPlotUrls,
    plotServerPort: () => state.plotServerPort,
  };
});
