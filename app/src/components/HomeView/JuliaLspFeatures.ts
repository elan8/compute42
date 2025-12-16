import * as monaco from 'monaco-editor';
import type { editor, IDisposable } from 'monaco-editor';
import { lspService } from '../../services/lspService';
import { debug } from '../../utils/logger';

export interface JuliaLspFeaturesOptions {
  editor: editor.IStandaloneCodeEditor;
  language: string;
  projectRoot?: string | null;
}

export class JuliaLspFeatures {
  // editor reference currently unused; reserved for future features
  private language: string;
  private projectRoot?: string | null;
  private hoverProvider: IDisposable | null = null;
  private completionProvider: IDisposable | null = null;
  private diagnosticsProvider: IDisposable | null = null;

  constructor(options: JuliaLspFeaturesOptions) {
    // editor not stored; lifecycle handled by Monaco providers
    this.language = options.language;
    this.projectRoot = options.projectRoot;

    debug(`JuliaLspFeatures: Initializing with language: ${this.language}`);

    if (this.language === 'julia') {
      this.initializeProviders();
    }
  }

  // Setup hover provider
  private setupHoverProvider(): void {
    if (this.language !== 'julia') {
      return;
    }

    debug('JuliaLspFeatures: Setting up hover provider');

    this.hoverProvider = monaco.languages.registerHoverProvider('julia', {
      provideHover: async (model, position, _token) => {
        try {
          const uri = model.uri.toString();
          debug(
            `JuliaLspFeatures: Hover requested for ${uri} at L${position.lineNumber}C${position.column}`
          );

          const hover = await lspService.requestHover(
            uri,
            position.lineNumber - 1,
            position.column - 1
          );

          if (hover && hover.contents && hover.contents.length > 0) {
            debug(`JuliaLspFeatures: Hover response received with content`);
            const contents = hover.contents.map((content) => ({
              value: content.value,
              isTrusted: true,
            }));

            return {
              contents: contents,
              range: hover.range
                ? {
                    startLineNumber: hover.range.start.line + 1,
                    startColumn: hover.range.start.character + 1,
                    endLineNumber: hover.range.end.line + 1,
                    endColumn: hover.range.end.character + 1,
                  }
                : undefined,
            };
          } else {
            debug(`JuliaLspFeatures: No hover content received`);
          }
        } catch (err) {
          debug(`JuliaLspFeatures: Failed to get hover: ${err}`);
        }

        return null;
      },
    });
  }

  // Setup completion provider
  private setupCompletionProvider(): void {
    if (this.language !== 'julia') {
      return;
    }

    debug('JuliaLspFeatures: Setting up completion provider');

    this.completionProvider = monaco.languages.registerCompletionItemProvider('julia', {
      triggerCharacters: ['.', ':', '"', "'", '(', '[', ','],
      provideCompletionItems: async (model, position, _context, _token) => {
        try {
          const uri = model.uri.toString();
          const languageId = model.getLanguageId();
          const lineContent1 = model.getLineContent(position.lineNumber);
          let lineContent0 = '';
          try {
            lineContent0 = model.getLineContent(position.lineNumber - 1);
          } catch {}
          const prefix = lineContent1.slice(0, Math.max(0, position.column - 1));
          const fileContentAll = model.getValue();
          debug(
            `[frontend] Completion: lang=${languageId} uri=${uri} at L${position.lineNumber}C${position.column} ` +
              `lineContent(pos.lineNumber)='${lineContent1}' lineContent(pos.lineNumber-1)='${lineContent0}' ` +
              `prefix='${prefix}' fileContentSnip='${fileContentAll.slice(0, 40).replace(/\n/g, '\\n')}'`
          );
          const startedAt = Date.now();
          const completion = await lspService.requestCompletion(
            uri,
            position.lineNumber - 1,
            position.column - 1,
            model.getValue()
          );
          const elapsedMs = Date.now() - startedAt;
          if (completion && completion.items) {
            debug(
              `JuliaLspFeatures: Completion response items=${completion.items.length} timeMs=${elapsedMs}`
            );
            return {
              suggestions: completion.items.map((item: any) => ({
                label: item.label,
                kind: item.kind ?? monaco.languages.CompletionItemKind.Text,
                detail: item.detail ?? undefined,
                documentation: item.documentation ?? undefined,
                insertText: item.insert_text ?? item.label,
                sortText: item.sort_text ?? undefined,
                filterText: item.filter_text ?? undefined,
                range: {
                  startLineNumber: position.lineNumber,
                  startColumn: position.column,
                  endLineNumber: position.lineNumber,
                  endColumn: position.column,
                },
              })),
            } as monaco.languages.CompletionList;
          }
          debug(`JuliaLspFeatures: No completion items returned timeMs=${elapsedMs}`);
        } catch (err) {
          debug(`JuliaLspFeatures: Failed to get completion: ${err}`);
        }

        return { suggestions: [] };
      },
    });
  }

  // Setup diagnostics provider (for future use)
  private setupDiagnosticsProvider(): void {
    if (this.language !== 'julia') {
      return;
    }

    debug('JuliaLspFeatures: Setting up diagnostics provider');

    // This will be implemented when we add diagnostics support
    // For now, just log that it's set up
  }

  // Initialize all LSP providers
  private initializeProviders(): void {
    if (this.language === 'julia') {
      debug('JuliaLspFeatures: Initializing LSP providers for Julia');
      this.setupHoverProvider();
      this.setupCompletionProvider();
      this.setupDiagnosticsProvider();
    }
  }

  // Update project root
  public updateProjectRoot(newProjectRoot: string | null): void {
    if (newProjectRoot !== this.projectRoot && this.language === 'julia') {
      debug(
        `JuliaLspFeatures: Project root changed to ${newProjectRoot}, reinitializing providers`
      );
      this.projectRoot = newProjectRoot;
      this.cleanupProviders();
      this.initializeProviders();
    }
  }

  // Update language
  public updateLanguage(newLanguage: string): void {
    if (newLanguage !== this.language) {
      debug(`JuliaLspFeatures: Language changed from ${this.language} to ${newLanguage}`);
      this.language = newLanguage;
      this.cleanupProviders();
      if (newLanguage === 'julia') {
        this.initializeProviders();
      }
    }
  }

  // Cleanup providers
  private cleanupProviders(): void {
    if (this.hoverProvider) {
      this.hoverProvider.dispose();
      this.hoverProvider = null;
    }
    if (this.completionProvider) {
      this.completionProvider.dispose();
      this.completionProvider = null;
    }
    if (this.diagnosticsProvider) {
      this.diagnosticsProvider.dispose();
      this.diagnosticsProvider = null;
    }
  }

  // Dispose all resources
  public dispose(): void {
    debug('JuliaLspFeatures: Disposing providers');
    this.cleanupProviders();
  }
}
