import { createApp, h, defineComponent } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import { create, NConfigProvider, darkTheme } from 'naive-ui';
import { themeOverrides } from './theme';
import hljs from 'highlight.js/lib/core';
import rust from 'highlight.js/lib/languages/rust';
import plaintext from 'highlight.js/lib/languages/plaintext';
import router from './router';
import { usePlotStore } from './store/plotStore';
import { debug } from './utils/logger';
import { startPlotNavigatorBridge } from './services/plotEventsBridge';
import { startLspStatusBridge } from './services/lspEventsBridge';
import { startOrchestratorEventsBridge } from './services/orchestratorEventsBridge';
// FontAwesome removed - now using Ionicons

// General Fonts
import 'vfonts/Lato.css';
// Monospace Fonts
import 'vfonts/FiraCode.css';
// Ionicons CSS will be imported by the icon components

// Register languages we need
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('text', plaintext);

const naive = create();
const pinia = createPinia();

const RootComponent = defineComponent({
  render() {
    return h(
      NConfigProvider,
      { theme: darkTheme, themeOverrides: themeOverrides, hljs: hljs },
      { default: () => h(App) }
    );
  },
});

const app = createApp(RootComponent);

app.use(naive);
app.use(pinia);
app.use(router);

// FontAwesome component registration removed - now using Ionicons

// Initialize plot store early to ensure global plot listening is set up
// Initialize plot store to register event listeners
usePlotStore();
void debug('Main: Plot store initialized');

app.mount('#app');

window.addEventListener('beforeunload', async () => {});

// Phase 5: start event bridges (plots, LSP, orchestrator)
void startPlotNavigatorBridge();
void startLspStatusBridge();
void startOrchestratorEventsBridge();
