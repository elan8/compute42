import { unifiedEventService, EventCategory } from './unifiedEventService';
import { appEventBus } from './eventBus';

/** Bridge Tauri plot events into the typed appEventBus */
export async function startPlotNavigatorBridge(): Promise<void> {
  await unifiedEventService.addEventListener(
    EventCategory.Plot,
    'navigator-update',
    async (evt) => {
      const ids = Array.isArray(evt.payload) ? (evt.payload as string[]) : [];
      appEventBus.emit('plot:navigator-update', ids);
    }
  );
}
