import { ref, onMounted, onUnmounted } from 'vue';
import { appEventBus } from '../../services/eventBus';

type LspStatus = {
  status: string;
  message?: string;
  error?: string | null;
  project_path?: string | null;
};

export function useLspStatus() {
  const status = ref<LspStatus | null>(null);

  function handler(payload: LspStatus) {
    status.value = payload;
  }

  onMounted(() => {
    appEventBus.on('lsp:status', handler);
  });

  onUnmounted(() => {
    appEventBus.off('lsp:status', handler);
  });

  return { status };
}
