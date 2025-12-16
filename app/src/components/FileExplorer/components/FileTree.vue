<template>
  <div class="file-tree-container">
    <n-tree
      v-if="data.length > 0"
      block-line
      :data="data as any"
      key-field="path"
      label-field="name"
      children-field="children"
      selectable
      :expanded-keys="expandedKeys"
      :selected-keys="selectedKeys"
      :on-load="handleLoad"
      :node-props="nodeProps"
      @update:expanded-keys="handleExpandedKeysChange"
      @update:selected-keys="handleSelectedKeysChange"
    >
    </n-tree>

    <n-empty
      v-else-if="!loading && !error"
      description="Select a folder or folder is empty"
      style="margin-top: 10px"
    />

    <n-alert
      v-if="error"
      type="error"
      title="Error"
      closable
      @close="$emit('error-clear')"
      style="margin: 8px"
    >
      {{ error }}
    </n-alert>
  </div>
</template>

<script setup lang="ts">
import { NTree, NEmpty, NAlert } from 'naive-ui';
import type { TreeOption } from 'naive-ui';
import { watch } from 'vue';
import { debug } from '../../../utils/logger';

// Import types
import type { FileNode } from '../../../types/fileTree';

// ============================
// Props
// ============================

interface Props {
  data: FileNode[];
  expandedKeys: string[];
  selectedKeys: string[];
  loading?: boolean;
  error?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  error: null,
});

// ============================
// Emits
// ============================

const emit = defineEmits<{
  'node-select': [node: FileNode];
  'node-expand': [node: FileNode];
  'node-context-menu': [node: FileNode | null, event: MouseEvent];
  'expanded-keys-change': [keys: string[]];
  'selected-keys-change': [keys: string[]];
  'error-clear': [];
  'node-load': [node: FileNode, resolve: () => void];
}>();

// ============================
// Event Handlers
// ============================

const handleExpandedKeysChange = (keys: string[]) => {
  emit('expanded-keys-change', keys);

  // Find newly expanded keys and emit node-expand events
  const oldKeys = new Set(props.expandedKeys);
  const newKeys = keys.filter((key) => !oldKeys.has(key));

  // Handle expansion
  for (const key of newKeys) {
    const node = findNodeByKey(key);
    if (node) {
      emit('node-expand', node);
    }
  }
};

const handleSelectedKeysChange = (keys: string[], nodes: any[]) => {
  emit('selected-keys-change', keys);

  if (keys.length > 0 && nodes.length > 0) {
    emit('node-select', nodes[0]);
  }
};

const handleLoad = (node: TreeOption): Promise<void> => {
  // Find the corresponding FileNode
  const fileNode = findNodeByKey(node.key as string);
  if (!fileNode) {
    debug(`FileTree: No FileNode found for key: ${node.key}`);
    return Promise.resolve();
  }

  // Return a promise that resolves when the parent component finishes loading
  return new Promise<void>((resolve) => {
    // Emit the load event to the parent component with a resolve callback
    emit('node-load', fileNode, resolve);
  });
};

// Node props function for handling events
const nodeProps = ({ option }: { option: TreeOption }) => {
  return {
    onClick() {
      // The tree's built-in selection will handle this
    },
    onContextmenu(e: MouseEvent): void {
      // Find the FileNode from the option
      const fileNode = findNodeByKey(option.key as string);
      if (fileNode) {
        // Emit the context menu event to the parent component
        emit('node-context-menu', fileNode, e);

        e.preventDefault();
      }
    },
  };
};

// ============================
// Helper Functions
// ============================

const findNodeByKey = (key: string): FileNode | null => {
  const searchInNodes = (nodeList: FileNode[]): FileNode | null => {
    for (const node of nodeList) {
      if (node.path === key) {
        return node;
      }
      if (node.children) {
        const found = searchInNodes(node.children);
        if (found) return found;
      }
    }
    return null;
  };

  return searchInNodes(props.data);
};
</script>

<style scoped>
.file-tree-container {
  flex-grow: 1;
  overflow: auto;
  min-height: 0;
}

/* Removed duplicate rule - handled below */

/* Removed duplicate rule - handled below */

/* Removed duplicate rule - handled below */

:deep(.n-tree) {
  height: 100% !important;
  display: flex !important;
  flex-direction: column !important;
}

:deep(.n-tree .n-tree-wrapper) {
  flex-grow: 1 !important;
  overflow: visible !important;
}

:deep(.n-tree-node-content) {
  min-width: 0 !important;
  overflow: hidden !important;
}

/* VS Code-like indentation - reduce spacing for subfolders and files */
:deep(.n-tree-node-indent) {
  width: 16px !important; /* Reduce from default ~20px to 16px like VS Code */
}

:deep(.n-tree-node-indent .n-tree-node-indent) {
  width: 16px !important; /* Consistent indentation for nested levels */
}

/* Reduce padding for tree nodes to match VS Code's compact look */
:deep(.n-tree .n-tree-node) {
  padding-top: 0px !important;
  padding-bottom: 0px !important;
  height: 22px !important; /* Match VS Code's 22px height */
  line-height: 22px !important;
}

/* Reduce spacing between tree node content */
:deep(.n-tree-node-content) {
  padding-left: 4px !important; /* Minimal left padding like VS Code */
  padding-right: 4px !important;
  display: flex !important;
  align-items: center !important; /* Center align all content vertically */
  height: 22px !important; /* Match the node height */
}

/* Ensure icons are properly sized and aligned */
:deep(.n-tree-node-content__prefix) {
  margin-right: 6px !important; /* Space between icon and text */
  display: flex !important;
  align-items: center !important;
  justify-content: center !important;
  height: 16px !important; /* Fixed height for consistent alignment */
  width: 16px !important; /* Fixed width for consistent spacing */
}

/* Style for Ionicons (SVG icons) */
:deep(.n-tree-node-content__prefix svg) {
  width: 16px !important; /* Match VS Code's 16px icon size */
  height: 16px !important;
  display: block !important;
}

/* Reduce twistie (expand/collapse arrow) spacing */
:deep(.n-tree-node-switcher) {
  width: 16px !important; /* Match VS Code's twistie width */
  margin-right: 2px !important;
  display: flex !important;
  align-items: center !important;
  justify-content: center !important;
}

/* Ensure text doesn't wrap and handles overflow properly */
:deep(.n-tree-node-content__text) {
  overflow: hidden !important;
  text-overflow: ellipsis !important;
  white-space: nowrap !important;
  flex: 1 !important;
  display: flex !important;
  align-items: center !important; /* Center text vertically */
  height: 22px !important; /* Match the content height */
  line-height: 1 !important; /* Reset line height for better control */
}
</style>
