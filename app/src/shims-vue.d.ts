declare module '*.vue' {
  import type { DefineComponent } from 'vue';
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module './providers/HoverProvider.vue' {
  import type { DefineComponent } from 'vue';
  import type * as monaco from 'monaco-editor';

  interface HoverProviderProps {
    editor: monaco.editor.IStandaloneCodeEditor;
    language: string;
  }

  const component: DefineComponent<HoverProviderProps, {}, any>;
  export default component;
}

declare module './providers/InlayHintsProvider.vue' {
  import type { DefineComponent } from 'vue';
  import type * as monaco from 'monaco-editor';

  interface InlayHintsProviderProps {
    editor: monaco.editor.IStandaloneCodeEditor;
    language: string;
    projectRoot: string | null;
  }

  const component: DefineComponent<InlayHintsProviderProps, {}, any>;
  export default component;
}

declare module './providers/CompletionProvider.vue' {
  import type { DefineComponent } from 'vue';
  import type * as monaco from 'monaco-editor';

  interface CompletionProviderProps {
    editor: monaco.editor.IStandaloneCodeEditor;
    language: string;
    projectRoot: string | null;
  }

  const component: DefineComponent<CompletionProviderProps, {}, any>;
  export default component;
}
