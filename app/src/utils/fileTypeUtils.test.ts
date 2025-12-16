import { describe, it, expect } from 'vitest';
import {
  getFileTypeInfo,
  isImageFile,
  isTextFile,
  isDocumentFile,
  isCsvFile,
  getLanguageFromPath,
  getViewerType,
} from './fileTypeUtils';

describe('fileTypeUtils', () => {
  describe('getFileTypeInfo', () => {
    it('should identify Julia files', () => {
      const info = getFileTypeInfo('test.jl');
      expect(info.type).toBe('text');
      expect(info.language).toBe('julia');
      expect(info.viewer).toBe('monaco');
    });

    it('should identify TypeScript files', () => {
      const info = getFileTypeInfo('test.ts');
      expect(info.type).toBe('text');
      expect(info.language).toBe('typescript');
      expect(info.viewer).toBe('monaco');
    });

    it('should identify JavaScript files', () => {
      const info = getFileTypeInfo('test.js');
      expect(info.type).toBe('text');
      expect(info.language).toBe('javascript');
      expect(info.viewer).toBe('monaco');
    });

    it('should identify JSON files', () => {
      const info = getFileTypeInfo('package.json');
      expect(info.type).toBe('text');
      expect(info.language).toBe('json');
      expect(info.viewer).toBe('monaco');
    });

    it('should identify Markdown files', () => {
      const info = getFileTypeInfo('README.md');
      expect(info.type).toBe('text');
      expect(info.language).toBe('markdown');
      expect(info.viewer).toBe('monaco');
    });

    it('should identify CSV files', () => {
      const info = getFileTypeInfo('data.csv');
      expect(info.type).toBe('csv');
      expect(info.viewer).toBe('csv');
    });

    it('should identify image files', () => {
      const jpgInfo = getFileTypeInfo('image.jpg');
      expect(jpgInfo.type).toBe('image');
      expect(jpgInfo.viewer).toBe('image');

      const pngInfo = getFileTypeInfo('image.png');
      expect(pngInfo.type).toBe('image');
      expect(pngInfo.viewer).toBe('image');
    });

    it('should identify PDF files', () => {
      const info = getFileTypeInfo('document.pdf');
      expect(info.type).toBe('document');
      expect(info.viewer).toBe('document');
    });

    it('should identify Jupyter notebook files', () => {
      const info = getFileTypeInfo('notebook.ipynb');
      expect(info.type).toBe('notebook');
      expect(info.viewer).toBe('notebook');
    });

    it('should handle files without extensions as plaintext', () => {
      const info = getFileTypeInfo('LICENSE');
      expect(info.type).toBe('text');
      expect(info.language).toBe('plaintext');
      expect(info.viewer).toBe('monaco');
    });

    it('should handle dotfiles as plaintext', () => {
      const info = getFileTypeInfo('.gitignore');
      expect(info.type).toBe('text');
      expect(info.language).toBe('plaintext');
      expect(info.viewer).toBe('monaco');
    });

    it('should handle common ASCII files without extensions', () => {
      const readmeInfo = getFileTypeInfo('README');
      expect(readmeInfo.type).toBe('text');
      expect(readmeInfo.language).toBe('plaintext');

      const makefileInfo = getFileTypeInfo('Makefile');
      expect(makefileInfo.type).toBe('text');
      expect(makefileInfo.language).toBe('plaintext');
    });

    it('should handle unknown file types as binary', () => {
      const info = getFileTypeInfo('unknown.xyz');
      expect(info.type).toBe('binary');
      expect(info.viewer).toBe('binary');
    });

    it('should handle case-insensitive extensions', () => {
      const upperInfo = getFileTypeInfo('TEST.JL');
      expect(upperInfo.language).toBe('julia');

      const mixedInfo = getFileTypeInfo('Test.Jl');
      expect(mixedInfo.language).toBe('julia');
    });

    it('should handle paths with directories', () => {
      const info = getFileTypeInfo('/path/to/file.jl');
      expect(info.language).toBe('julia');
    });
  });

  describe('isImageFile', () => {
    it('should return true for image files', () => {
      expect(isImageFile('image.jpg')).toBe(true);
      expect(isImageFile('image.png')).toBe(true);
      expect(isImageFile('image.gif')).toBe(true);
    });

    it('should return false for non-image files', () => {
      expect(isImageFile('file.jl')).toBe(false);
      expect(isImageFile('file.txt')).toBe(false);
    });
  });

  describe('isTextFile', () => {
    it('should return true for text files', () => {
      expect(isTextFile('file.jl')).toBe(true);
      expect(isTextFile('file.ts')).toBe(true);
      expect(isTextFile('file.md')).toBe(true);
    });

    it('should return false for non-text files', () => {
      expect(isTextFile('image.jpg')).toBe(false);
      expect(isTextFile('data.csv')).toBe(false);
    });
  });

  describe('isDocumentFile', () => {
    it('should return true for document files', () => {
      expect(isDocumentFile('document.pdf')).toBe(true);
    });

    it('should return false for non-document files', () => {
      expect(isDocumentFile('file.jl')).toBe(false);
      expect(isDocumentFile('image.jpg')).toBe(false);
    });
  });

  describe('isCsvFile', () => {
    it('should return true for CSV files', () => {
      expect(isCsvFile('data.csv')).toBe(true);
    });

    it('should return false for non-CSV files', () => {
      expect(isCsvFile('file.jl')).toBe(false);
      expect(isCsvFile('image.jpg')).toBe(false);
    });
  });

  describe('getLanguageFromPath', () => {
    it('should return correct language for known file types', () => {
      expect(getLanguageFromPath('file.jl')).toBe('julia');
      expect(getLanguageFromPath('file.ts')).toBe('typescript');
      expect(getLanguageFromPath('file.js')).toBe('javascript');
      expect(getLanguageFromPath('file.md')).toBe('markdown');
    });

    it('should return plaintext for unknown file types', () => {
      expect(getLanguageFromPath('unknown.xyz')).toBe('plaintext');
    });
  });

  describe('getViewerType', () => {
    it('should return correct viewer type', () => {
      expect(getViewerType('file.jl')).toBe('monaco');
      expect(getViewerType('image.jpg')).toBe('image');
      expect(getViewerType('document.pdf')).toBe('document');
      expect(getViewerType('data.csv')).toBe('csv');
      expect(getViewerType('notebook.ipynb')).toBe('notebook');
      expect(getViewerType('unknown.xyz')).toBe('binary');
    });
  });
});


