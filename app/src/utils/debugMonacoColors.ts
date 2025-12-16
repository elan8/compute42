/**
 * Debug utility to extract actual colors from Monaco editor
 * This helps us see what colors Monaco is actually using for Julia syntax
 *
 * Usage: Call `window.debugMonacoColors()` in browser console when a Julia file is open
 */

import * as monaco from 'monaco-editor';
import { debug as logDebug, info as logInfo } from '@tauri-apps/plugin-log';

/**
 * Extract actual token colors from Monaco editor for debugging
 * This inspects the computed styles of tokens in the editor
 */
export async function debugMonacoColors(
  editor: monaco.editor.IStandaloneCodeEditor
): Promise<Record<string, string>> {
  await logInfo('=== Starting Monaco Color Extraction ===');

  const model = editor.getModel();
  if (!model || model.getLanguageId() !== 'julia') {
    await logInfo(`Editor model is not Julia language (got: ${model?.getLanguageId() || 'null'})`);
    return {};
  }

  await logInfo(`Analyzing model: ${model.uri.toString()}`);
  await logInfo(`Language: ${model.getLanguageId()}`);
  await logInfo(`Line count: ${model.getLineCount()}`);

  // Get the editor DOM element
  const domNode = editor.getDomNode();
  if (!domNode) {
    await logInfo('Could not find editor DOM node');
    return {};
  }

  // Find all token spans in the editor
  const tokenSpans = domNode.querySelectorAll(
    '.mtk1, .mtk2, .mtk3, .mtk4, .mtk5, .mtk6, .mtk7, .mtk8, .mtk9, .mtk10, .mtk11, .mtk12, .mtk13, .mtk14, .mtk15, .mtk16, .mtk17, .mtk18, .mtk19, .mtk20'
  );

  const colorMap = new Map<string, Set<string>>();

  tokenSpans.forEach((span) => {
    const element = span as HTMLElement;
    const computedStyle = window.getComputedStyle(element);
    const color = computedStyle.color;
    const classes = Array.from(element.classList);

    // Find the token class (mtk1, mtk2, etc.)
    const tokenClass = classes.find((cls) => cls.startsWith('mtk'));

    if (tokenClass) {
      if (!colorMap.has(tokenClass)) {
        colorMap.set(tokenClass, new Set());
      }
      colorMap.get(tokenClass)!.add(color);
    }
  });

  const result: Record<string, string> = {};

  await logInfo('=== Monaco Editor Token Colors ===');
  for (const [tokenClass, colors] of colorMap.entries()) {
    const colorArray = Array.from(colors);
    await logInfo(`${tokenClass}: ${colorArray.join(', ')}`);
    if (colorArray.length > 0) {
      result[tokenClass] = colorArray[0];
    }
  }

  // Also try to get semantic token information by inspecting actual code
  const viewLines = domNode.querySelectorAll('.view-line');
  const semanticColors = new Map<string, string>();

  // Known Julia keywords to identify
  const juliaKeywords = [
    'function',
    'if',
    'else',
    'for',
    'while',
    'return',
    'using',
    'import',
    'export',
    'const',
    'let',
    'global',
    'local',
    'struct',
    'mutable',
    'abstract',
    'type',
    'end',
    'begin',
    'try',
    'catch',
    'finally',
  ];

  viewLines.forEach((line) => {
    const spans = line.querySelectorAll('span[class*="mtk"]');
    spans.forEach((span) => {
      const element = span as HTMLElement;
      const text = element.textContent?.trim();
      const computedStyle = window.getComputedStyle(element);
      const color = computedStyle.color;
      const hexColor = rgbToHex(color);

      if (text && text.length > 0 && text.length < 30) {
        // Try to identify token type by content
        if (juliaKeywords.includes(text)) {
          if (!semanticColors.has('keyword')) {
            semanticColors.set('keyword', hexColor);
            result.keyword = hexColor;
          }
        } else if (
          text.startsWith('"') ||
          text.startsWith("'") ||
          (text.includes('"') && text.length > 1)
        ) {
          if (!semanticColors.has('string')) {
            semanticColors.set('string', hexColor);
            result.string = hexColor;
          }
        } else if (text.startsWith('#') || line.textContent?.trim().startsWith('#')) {
          if (!semanticColors.has('comment')) {
            semanticColors.set('comment', hexColor);
            result.comment = hexColor;
          }
        } else if (/^\d+/.test(text)) {
          if (!semanticColors.has('number')) {
            semanticColors.set('number', hexColor);
            result.number = hexColor;
          }
        } else if (/^[A-Z][a-zA-Z0-9]*$/.test(text)) {
          if (!semanticColors.has('type')) {
            semanticColors.set('type', hexColor);
            result.type = hexColor;
          }
        } else if (/^[a-z][a-zA-Z0-9_]*$/.test(text)) {
          // Could be function or variable - check if it's followed by (
          const nextSibling = element.nextElementSibling;
          if (nextSibling && nextSibling.textContent?.trim().startsWith('(')) {
            if (!semanticColors.has('function')) {
              semanticColors.set('function', hexColor);
              result.function = hexColor;
            }
          } else if (!semanticColors.has('variable')) {
            semanticColors.set('variable', hexColor);
            result.variable = hexColor;
          }
        }
      }
    });
  });

  await logInfo('=== Semantic Token Colors (extracted) ===');
  for (const [tokenType, color] of semanticColors.entries()) {
    await logInfo(`${tokenType}: ${color}`);
  }

  // Also try tokenization-based extraction
  const tokenColors = await extractMonacoColorsFromTokens(editor);
  Object.assign(result, tokenColors);

  await logInfo('=== Token-based Colors ===');
  for (const [key, value] of Object.entries(tokenColors)) {
    await logInfo(`${key}: ${value}`);
  }

  await logInfo('=== Color Scheme Update (copy to juliaColorScheme.ts) ===');
  const colorSchemeUpdate: Partial<Record<keyof typeof result, string>> = {};
  if (result.keyword) colorSchemeUpdate.keyword = result.keyword;
  if (result.string) colorSchemeUpdate.string = result.string;
  if (result.comment) colorSchemeUpdate.comment = result.comment;
  if (result.number) colorSchemeUpdate.number = result.number;
  if (result.function) colorSchemeUpdate.function = result.function;
  if (result.variable) colorSchemeUpdate.variable = result.variable;
  if (result.type) colorSchemeUpdate.type = result.type;

  await logInfo(JSON.stringify(colorSchemeUpdate, null, 2));
  await logInfo('💡 Update juliaColors object in juliaColorScheme.ts with these values');
  await logInfo('=== Monaco Color Extraction Complete ===');

  return result;
}

/**
 * Get a sample of actual colors from Monaco editor by inspecting rendered tokens
 * Returns a map of token types to their actual colors
 */
export function getMonacoActualColors(
  editor: monaco.editor.IStandaloneCodeEditor
): Map<string, string> {
  const colors = new Map<string, string>();
  const domNode = editor.getDomNode();

  if (!domNode) {
    return colors;
  }

  // Sample code to help identify token types
  const sampleCode = `
function example(x::Int64)::String
    # This is a comment
    y = "string value"
    z = 42
    return y
end
`;

  // We'll need to inspect the actual rendered tokens
  // This is a simplified approach - in practice, we'd need to parse the token stream
  const viewLines = domNode.querySelectorAll('.view-line');

  viewLines.forEach((line) => {
    const tokens = line.querySelectorAll('span[class*="mtk"]');
    tokens.forEach((token) => {
      const element = token as HTMLElement;
      const text = element.textContent?.trim() || '';
      const computedColor = window.getComputedStyle(element).color;

      // Try to categorize based on text content
      if (text === 'function' || text === 'end' || text === 'return') {
        if (!colors.has('keyword')) {
          colors.set('keyword', computedColor);
        }
      } else if (text.startsWith('#')) {
        if (!colors.has('comment')) {
          colors.set('comment', computedColor);
        }
      } else if (text.startsWith('"') || text.startsWith("'")) {
        if (!colors.has('string')) {
          colors.set('string', computedColor);
        }
      } else if (/^\d+$/.test(text)) {
        if (!colors.has('number')) {
          colors.set('number', computedColor);
        }
      } else if (/^[A-Z][a-zA-Z0-9]*$/.test(text)) {
        if (!colors.has('type')) {
          colors.set('type', computedColor);
        }
      }
    });
  });

  return colors;
}

/**
 * Helper to convert RGB color to hex
 */
function rgbToHex(rgb: string): string {
  const match = rgb.match(/^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/);
  if (!match) {
    return rgb; // Already hex or invalid
  }

  const r = parseInt(match[1], 10);
  const g = parseInt(match[2], 10);
  const b = parseInt(match[3], 10);

  return (
    '#' +
    [r, g, b]
      .map((x) => {
        const hex = x.toString(16);
        return hex.length === 1 ? '0' + hex : hex;
      })
      .join('')
  );
}

/**
 * Extract colors using Monaco's tokenization API
 * This is the most accurate method as it uses Monaco's internal token information
 */
export async function extractMonacoColorsFromTokens(
  editor: monaco.editor.IStandaloneCodeEditor
): Promise<Record<string, string>> {
  const model = editor.getModel();
  if (!model || model.getLanguageId() !== 'julia') {
    await logInfo('Editor model is not Julia language');
    return {};
  }

  const colors: Record<string, string> = {};
  const domNode = editor.getDomNode();

  if (!domNode) {
    return {};
  }

  // Get tokens for each line (Monaco tokenizes automatically)
  for (let lineNumber = 1; lineNumber <= model.getLineCount(); lineNumber++) {
    const lineTokens = model.getLineTokens(lineNumber);
    const lineContent = model.getLineContent(lineNumber);

    // Get the rendered line element
    const viewLines = domNode.querySelectorAll('.view-line');
    const viewLine = viewLines[lineNumber - 1] as HTMLElement | undefined;

    if (!viewLine) continue;

    // Map tokens to DOM elements
    const tokenSpans = viewLine.querySelectorAll('span[class*="mtk"]');
    let charOffset = 0;

    lineTokens.tokens.forEach((token, tokenIndex) => {
      const tokenText = lineContent.substring(token.startIndex, token.endIndex);
      const tokenType = token.type;

      // Find corresponding DOM element
      let spanIndex = 0;
      let currentOffset = 0;

      for (let i = 0; i < tokenSpans.length; i++) {
        const span = tokenSpans[i] as HTMLElement;
        const spanText = span.textContent || '';
        if (currentOffset + spanText.length > token.startIndex) {
          spanIndex = i;
          break;
        }
        currentOffset += spanText.length;
      }

      const span = tokenSpans[spanIndex] as HTMLElement | undefined;
      if (span) {
        const computedColor = window.getComputedStyle(span).color;
        const hexColor = rgbToHex(computedColor);

        // Categorize based on token type and content
        const typeStr = tokenType;

        if (
          typeStr.includes('keyword') ||
          ['function', 'if', 'else', 'for', 'while', 'return', 'end', 'using', 'import'].includes(
            tokenText.trim()
          )
        ) {
          if (!colors.keyword) colors.keyword = hexColor;
        } else if (
          typeStr.includes('string') ||
          tokenText.includes('"') ||
          tokenText.includes("'")
        ) {
          if (!colors.string) colors.string = hexColor;
        } else if (typeStr.includes('comment') || tokenText.trim().startsWith('#')) {
          if (!colors.comment) colors.comment = hexColor;
        } else if (typeStr.includes('number') || /^\d+/.test(tokenText.trim())) {
          if (!colors.number) colors.number = hexColor;
        } else if (typeStr.includes('type') || /^[A-Z][a-zA-Z0-9]*$/.test(tokenText.trim())) {
          if (!colors.type) colors.type = hexColor;
        } else if (
          typeStr.includes('function') ||
          (tokenText.trim().match(/^[a-z][a-zA-Z0-9_]*$/) &&
            lineContent.includes(tokenText.trim() + '('))
        ) {
          if (!colors.function) colors.function = hexColor;
        } else if (typeStr.includes('variable') || /^[a-z][a-zA-Z0-9_]*$/.test(tokenText.trim())) {
          if (!colors.variable) colors.variable = hexColor;
        }
      }
    });
  }

  return colors;
}
