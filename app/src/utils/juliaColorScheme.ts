/**
 * Shared Julia syntax highlighting color scheme
 * Used by both Monaco Editor and CodeMirror to ensure consistent colors
 */

import type * as monaco from 'monaco-editor';

export interface JuliaColorScheme {
  keyword: string;
  string: string;
  comment: string;
  number: string;
  operator: string;
  punctuation: string;
  variable: string;
  function: string;
  type: string;
  namespace: string;
  macro: string;
  constant: string;
  property: string;
  attribute: string;
  definition: string;
  bracket: string;
  defaultText: string;
}

/**
 * Julia syntax highlighting colors matching actual Monaco editor appearance
 * Based on user observation: blue keywords, purple function names
 */
export const juliaColors: JuliaColorScheme = {
  keyword: '#569CD6', // Keywords (function, if, etc.) - BLUE
  string: '#CE9178', // Strings - orange/brown
  comment: '#6A9955', // Comments - green
  number: '#B5CEA8', // Numbers - light green
  operator: '#D4D4D4', // Operators - default text color (light gray)
  punctuation: '#D4D4D4', // Punctuation - default text color
  variable: '#D4D4D4', // Variables - default gray (matches Monaco)
  function: '#C586C0', // Functions - PURPLE/magenta
  type: '#4EC9B0', // Types - cyan
  namespace: '#4EC9B0', // Namespaces - cyan (like types)
  macro: '#569CD6', // Macros - blue (like keywords)
  constant: '#569CD6', // Constants - blue
  property: '#D4D4D4', // Properties - default gray (matches Monaco)
  attribute: '#D4D4D4', // Attributes - default gray (matches Monaco)
  definition: '#4FC1FF', // Definitions - bright blue
  bracket: '#D4D4D4', // Brackets - default text color
  defaultText: '#D4D4D4', // Default text color
};

/**
 * Get Monaco editor theme rules for Julia syntax highlighting
 * These rules override the base theme colors for specific token types
 */
export function getMonacoJuliaThemeRules(): monaco.editor.ITokenThemeRule[] {
  return [
    { token: 'keyword', foreground: juliaColors.keyword },
    { token: 'string', foreground: juliaColors.string },
    { token: 'comment', foreground: juliaColors.comment, fontStyle: 'italic' },
    { token: 'number', foreground: juliaColors.number },
    { token: 'operator', foreground: juliaColors.operator },
    { token: 'punctuation', foreground: juliaColors.punctuation },
    { token: 'variable', foreground: juliaColors.variable },
    { token: 'variable.predefined', foreground: juliaColors.variable },
    { token: 'function', foreground: juliaColors.function },
    { token: 'function.name', foreground: juliaColors.function },
    { token: 'type', foreground: juliaColors.type },
    { token: 'type.name', foreground: juliaColors.type },
    { token: 'namespace', foreground: juliaColors.namespace },
    { token: 'macro', foreground: juliaColors.macro },
    { token: 'constant', foreground: juliaColors.constant },
    { token: 'property', foreground: juliaColors.property },
    { token: 'attribute', foreground: juliaColors.attribute },
  ];
}
