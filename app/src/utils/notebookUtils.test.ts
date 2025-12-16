import { describe, it, expect } from 'vitest';
import { createEmptyJuliaNotebook } from './notebookUtils';

describe('notebookUtils', () => {
  describe('createEmptyJuliaNotebook', () => {
    it('should create a valid Jupyter notebook JSON structure', () => {
      const notebookJson = createEmptyJuliaNotebook();
      const notebook = JSON.parse(notebookJson);

      expect(notebook).toHaveProperty('nbformat');
      expect(notebook).toHaveProperty('nbformat_minor');
      expect(notebook).toHaveProperty('metadata');
      expect(notebook).toHaveProperty('cells');
    });

    it('should have correct nbformat version', () => {
      const notebookJson = createEmptyJuliaNotebook();
      const notebook = JSON.parse(notebookJson);

      expect(notebook.nbformat).toBe(4);
      expect(notebook.nbformat_minor).toBe(4);
    });

    it('should have Julia kernel metadata', () => {
      const notebookJson = createEmptyJuliaNotebook();
      const notebook = JSON.parse(notebookJson);

      expect(notebook.metadata).toHaveProperty('kernelspec');
      expect(notebook.metadata.kernelspec).toHaveProperty('display_name');
      expect(notebook.metadata.kernelspec).toHaveProperty('language');
      expect(notebook.metadata.kernelspec).toHaveProperty('name');

      expect(notebook.metadata.kernelspec.display_name).toBe('Julia 1.9+');
      expect(notebook.metadata.kernelspec.language).toBe('julia');
      expect(notebook.metadata.kernelspec.name).toBe('julia-1.9');
    });

    it('should have language_info metadata', () => {
      const notebookJson = createEmptyJuliaNotebook();
      const notebook = JSON.parse(notebookJson);

      expect(notebook.metadata).toHaveProperty('language_info');
      expect(notebook.metadata.language_info).toHaveProperty('name');
      expect(notebook.metadata.language_info).toHaveProperty('version');

      expect(notebook.metadata.language_info.name).toBe('julia');
      expect(notebook.metadata.language_info.version).toBe('1.9');
    });

    it('should have an empty cells array', () => {
      const notebookJson = createEmptyJuliaNotebook();
      const notebook = JSON.parse(notebookJson);

      expect(notebook.cells).toEqual([]);
      expect(Array.isArray(notebook.cells)).toBe(true);
    });

    it('should return valid JSON string', () => {
      const notebookJson = createEmptyJuliaNotebook();
      expect(() => JSON.parse(notebookJson)).not.toThrow();
    });

    it('should be formatted with indentation', () => {
      const notebookJson = createEmptyJuliaNotebook();
      // Formatted JSON should have newlines
      expect(notebookJson).toContain('\n');
    });
  });
});


