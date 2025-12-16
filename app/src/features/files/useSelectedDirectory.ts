import { ref, onMounted, onUnmounted } from 'vue';
import { appEventBus } from '../../services/eventBus';

type SelectedDirectoryPayload = { path: string; is_julia_project: boolean };

export function useSelectedDirectory() {
  const selectedDirectory = ref<SelectedDirectoryPayload | null>(null);

  function handler(payload: SelectedDirectoryPayload) {
    selectedDirectory.value = payload;
  }

  onMounted(() => {
    appEventBus.on('orchestrator:selected-directory', handler);
  });

  onUnmounted(() => {
    appEventBus.off('orchestrator:selected-directory', handler);
  });

  return { selectedDirectory };
}
