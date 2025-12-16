import * as monaco from 'monaco-editor';

export interface Position {
  line: number;
  character: number;
}

export interface Range {
  start: Position;
  end: Position;
}

export interface TextDocumentIdentifier {
  uri: string;
}

export interface CompletionItem {
  label: string;
  kind?: number;
  detail?: string;
  documentation?: string | { kind: string; value: string };
  deprecated?: boolean;
  preselect?: boolean;
  sortText?: string;
  filterText?: string;
  insertText?: string;
  insertTextFormat?: number;
  textEdit?: {
    range: Range;
    newText: string;
  };
  additionalTextEdits?: {
    range: Range;
    newText: string;
  }[];
  commitCharacters?: string[];
  command?: {
    title: string;
    command: string;
    arguments?: any[];
  };
  data?: any;
}

export interface CompletionList {
  isIncomplete: boolean;
  items: CompletionItem[];
}

// Monaco editor specific types
export interface MonacoCompletionItem {
  label: string;
  kind?: monaco.languages.CompletionItemKind;
  detail?: string;
  documentation?: monaco.IMarkdownString | string;
  sortText?: string;
  filterText?: string;
  insertText?: string;
  insertTextRules?: monaco.languages.CompletionItemInsertTextRule;
  range?: monaco.Range;
  additionalTextEdits?: monaco.languages.TextEdit[];
  commitCharacters?: string[];
  command?: monaco.languages.Command;
  tags?: monaco.languages.CompletionItemTag[];
  preselect?: boolean;
}
