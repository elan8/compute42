import { unifiedEventService, EventCategory } from './unifiedEventService';
import { appEventBus } from './eventBus';

/** Bridge Tauri LSP status events into the typed appEventBus */
export async function startLspStatusBridge(): Promise<void> {
  await unifiedEventService.addEventListener(EventCategory.Lsp, 'status', async (evt) => {
    appEventBus.emit('lsp:status', evt.payload);
  });
}
