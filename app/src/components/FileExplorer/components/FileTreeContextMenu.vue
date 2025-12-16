<template>
  <div
    v-if="visible"
    class="vscode-context-menu"
    :style="menuStyle"
    @click.stop
    @keydown.escape="handleEscape"
    tabindex="0"
  >
    <!-- Always available actions -->
    <div class="context-menu-item" @click="handleAction('new-julia-file')">
      <div class="context-menu-icon">
        <n-icon><DocumentOutline /></n-icon>
      </div>
      <span class="context-menu-label">New Julia File</span>
    </div>

    <div class="context-menu-item" @click="handleAction('new-jupyter-notebook')">
      <div class="context-menu-icon">
        <n-icon><BookOutline /></n-icon>
      </div>
      <span class="context-menu-label">New Jupyter Notebook</span>
    </div>

    <div class="context-menu-item" @click="handleAction('new-folder')">
      <div class="context-menu-icon">
        <n-icon><FolderOutline /></n-icon>
      </div>
      <span class="context-menu-label">New Folder</span>
    </div>

    <!-- Target-specific actions -->
    <template v-if="target">
      <div class="context-menu-separator"></div>

      <div class="context-menu-item" @click="handleAction('rename')">
        <div class="context-menu-icon">
          <n-icon><CreateOutline /></n-icon>
        </div>
        <span class="context-menu-label">Rename</span>
      </div>

      <div class="context-menu-item context-menu-item-danger" @click="handleAction('delete')">
        <div class="context-menu-icon">
          <n-icon><TrashOutline /></n-icon>
        </div>
        <span class="context-menu-label">Delete</span>
      </div>

      <!-- File-specific actions -->
      <template v-if="!target.is_directory">
        <div class="context-menu-separator"></div>

        <div class="context-menu-item" @click="handleAction('copy-path')">
          <div class="context-menu-icon">
            <n-icon><CopyOutline /></n-icon>
          </div>
          <span class="context-menu-label">Copy Path</span>
        </div>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { NIcon } from 'naive-ui';
import {
  DocumentOutline,
  BookOutline,
  FolderOutline,
  CreateOutline,
  TrashOutline,
  CopyOutline,
} from '@vicons/ionicons5';
import { debug } from '../../../utils/logger';

// Import types
import type { FileNode } from '../../../types/fileTree';

// ============================
// Props
// ============================

interface Props {
  target: FileNode | null;
  position: { x: number; y: number };
  visible: boolean;
}

const props = defineProps<Props>();

// ============================
// Emits
// ============================

const emit = defineEmits<{
  action: [action: string];
  close: [];
}>();

// ============================
// Computed Properties
// ============================

const menuStyle = computed(() => {
  const menuWidth = 180;
  const menuHeight = 200; // Approximate height
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  let left = props.position.x;
  let top = props.position.y;

  // Adjust horizontal position if menu would go off-screen
  if (left + menuWidth > windowWidth) {
    left = windowWidth - menuWidth - 10;
  }

  // Adjust vertical position if menu would go off-screen
  if (top + menuHeight > windowHeight) {
    top = windowHeight - menuHeight - 10;
  }

  return {
    left: left + 'px',
    top: top + 'px',
  };
});

// ============================
// Event Handlers
// ============================

const handleAction = (action: string) => {
  debug(`FileTreeContextMenu: Action triggered: ${action}`);
  emit('action', action);
};

const handleEscape = () => {
  debug('FileTreeContextMenu: Escape key pressed, closing menu');
  emit('close');
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as Element;
  const contextMenu = target.closest('.vscode-context-menu');

  // Close if clicking outside the context menu
  if (!contextMenu) {
    debug('FileTreeContextMenu: Clicking outside, closing menu');
    emit('close');
  }
};

// ============================
// Lifecycle
// ============================

onMounted(() => {
  debug('FileTreeContextMenu: Component mounted, adding event listeners');
  document.addEventListener('click', handleClickOutside);
});

onUnmounted(() => {
  debug('FileTreeContextMenu: Component unmounted, removing event listeners');
  document.removeEventListener('click', handleClickOutside);
});
</script>

<style scoped>
/* VS Code-like Context Menu Styles */
.vscode-context-menu {
  position: fixed;
  z-index: 99999;
  background: #2d2d30;
  border: 1px solid #3e3e42;
  border-radius: 6px;
  box-shadow:
    0 8px 16px rgba(0, 0, 0, 0.4),
    0 2px 4px rgba(0, 0, 0, 0.2);
  padding: 4px 0;
  min-width: 180px;
  font-family:
    -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell',
    sans-serif;
  font-size: 13px;
  color: #cccccc;
  user-select: none;
  outline: none;
  backdrop-filter: blur(10px);
  pointer-events: auto;
}

.context-menu-item {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
  border-radius: 3px;
  margin: 0 4px;
}

.context-menu-item:hover {
  background-color: #094771;
  transform: translateX(1px);
}

.context-menu-item:active {
  background-color: #007acc;
  transform: translateX(1px);
}

.context-menu-item-danger:hover {
  background-color: #a1260d;
  transform: translateX(1px);
}

.context-menu-item-danger:active {
  background-color: #c42d1c;
  transform: translateX(1px);
}

.context-menu-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  margin-right: 8px;
  color: #cccccc;
  flex-shrink: 0;
}

.context-menu-item:hover .context-menu-icon {
  color: #ffffff;
}

.context-menu-item .n-icon {
  width: 16px;
  height: 16px;
}

.context-menu-label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-menu-separator {
  height: 1px;
  background-color: #3e3e42;
  margin: 4px 0;
}
</style>
