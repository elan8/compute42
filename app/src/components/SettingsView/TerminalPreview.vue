<template>
  <div ref="terminalContainer" class="terminal-preview-container"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue';
import { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { FitAddon } from '@xterm/addon-fit';

const props = defineProps({
  fontFamily: {
    type: String,
    default: null,
  },
  fontSize: {
    type: Number,
    default: null,
  },
});

const terminalContainer = ref<HTMLElement | null>(null);
let termInstance: Terminal | null = null;
let fitAddonInstance: FitAddon | null = null;

// Sample terminal content with ANSI color codes for green Julia prompts
// \x1b[1;32m = bold green, \x1b[0m = reset
const sampleTerminalContent = `\x1b[1;32mjulia> \x1b[0mprintln("Hello, Julia!")
Hello, Julia!

\x1b[1;32mjulia> \x1b[0mx = [1, 2, 3, 4, 5]
5-element Vector{Int64}:
 1
 2
 3
 4
 5

\x1b[1;32mjulia> \x1b[0msum(x)
15

\x1b[1;32mjulia> \x1b[0mfunction greet(name)
           println("Hello, ", name, "!")
       end
greet (generic function with 1 method)

\x1b[1;32mjulia> \x1b[0mgreet("Compute42")
Hello, Compute42!

\x1b[1;32mjulia> \x1b[0m`;

onMounted(async () => {
  await nextTick();

  if (terminalContainer.value) {
    // Create terminal instance with the exact same theme as the real terminal
    termInstance = new Terminal({
      cursorBlink: true,
      convertEol: true,
      fontFamily: props.fontFamily || '"Consolas", "Monaco", "Courier New", monospace',
      fontSize: props.fontSize || 13,
      theme: {
        background: '#1e1e1e',
        foreground: '#ffffff',
        cursor: '#ffffff',
        selection: '#264f78',
      },
      disableStdin: true, // Disable input for preview
      scrollback: 100, // Limit scrollback for preview
      allowTransparency: false, // Match real terminal settings
    });

    fitAddonInstance = new FitAddon();
    termInstance.loadAddon(fitAddonInstance);

    // Open terminal
    termInstance.open(terminalContainer.value);

    // Write sample content
    termInstance.write(sampleTerminalContent);

    // Fit terminal to container
    await nextTick();
    try {
      fitAddonInstance.fit();
    } catch (e) {
      console.warn('Failed to fit terminal on mount:', e);
    }
  }
});

// Watch for prop changes and update terminal
watch(
  () => [props.fontFamily, props.fontSize],
  async () => {
    if (termInstance) {
      termInstance.options.fontFamily =
        props.fontFamily || '"Consolas", "Monaco", "Courier New", monospace';
      termInstance.options.fontSize = props.fontSize || 13;

      // Refit terminal after font changes
      await nextTick();
      if (fitAddonInstance) {
        try {
          fitAddonInstance.fit();
        } catch (e) {
          console.warn('Failed to fit terminal after font change:', e);
        }
      }
    }
  }
);

onBeforeUnmount(() => {
  if (termInstance) {
    termInstance.dispose();
    termInstance = null;
  }
  fitAddonInstance = null;
});
</script>

<style scoped>
.terminal-preview-container {
  width: 100%;
  height: 400px;
  min-height: 300px;
  border: 1px solid var(--n-border-color);
  border-radius: 4px;
  overflow: hidden;
  background: #1e1e1e;
}

.terminal-preview-container :deep(.xterm) {
  height: 100%;
  color: #ffffff; /* Ensure text color matches real terminal */
}

.terminal-preview-container :deep(.xterm-viewport) {
  background: #1e1e1e !important;
}

.terminal-preview-container :deep(.xterm-screen) {
  color: #ffffff; /* Ensure text color matches real terminal foreground */
}

.terminal-preview-container :deep(.xterm-rows) {
  color: #ffffff; /* Ensure text color matches real terminal foreground */
}
</style>
