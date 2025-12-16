import { unifiedEventService, EventCategory } from './unifiedEventService';
import { appEventBus } from './eventBus';

/** Bridge orchestrator events into the typed appEventBus */
export async function startOrchestratorEventsBridge(): Promise<void> {
  // Project change status → app bus
  await unifiedEventService.addEventListener(
    EventCategory.Orchestrator,
    'project-change-status',
    async (evt) => {
      const payload = evt.payload as {
        message?: string;
        progress_percentage?: number;
      };
      if (payload && typeof payload === 'object') {
        appEventBus.emit('orchestrator:project-change-status', {
          message: payload.message ?? '',
          progress_percentage: payload.progress_percentage ?? 0,
        });
      }
    }
  );

  // Selected directory → app bus
  await unifiedEventService.addEventListener(
    EventCategory.Orchestrator,
    'selected-directory',
    async (evt) => {
      const payload = evt.payload as {
        path?: string;
        is_julia_project?: boolean;
      };
      if (payload && typeof payload === 'object') {
        appEventBus.emit('orchestrator:selected-directory', {
          path: payload.path ?? '',
          is_julia_project: !!payload.is_julia_project,
        });
      }
    }
  );
}
