import { ref, onMounted, onUnmounted } from 'vue';
import { appEventBus } from '../../services/eventBus';

type StatusPayload = { message?: string; progress_percentage?: number };

export function useProjectChangeStatus() {
  const status = ref<StatusPayload | null>(null);

  function handler(payload: StatusPayload) {
    status.value = payload;
  }

  let off: (() => void) | null = null;

  onMounted(() => {
    off = appEventBus.on('orchestrator:project-change-status', handler);
  });

  onUnmounted(() => {
    if (off) off();
  });

  return { status };
}
