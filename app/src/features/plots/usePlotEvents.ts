import { ref, onMounted, onUnmounted } from 'vue';
import { appEventBus } from '../../services/eventBus';

export function usePlotEvents() {
  const plotIds = ref<string[]>([]);

  function handler(ids: string[]) {
    plotIds.value = ids;
  }

  onMounted(() => {
    appEventBus.on('plot:navigator-update', handler);
  });

  onUnmounted(() => {
    appEventBus.off('plot:navigator-update', handler);
  });

  return { plotIds };
}
