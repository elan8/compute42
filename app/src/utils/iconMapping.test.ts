import { describe, it, expect } from 'vitest';
import { getViconsIcon, faToIonicons5, faToMaterial } from './iconMapping';

describe('iconMapping', () => {
  describe('getViconsIcon', () => {
    it('should return material icon for brand icons', () => {
      const result = getViconsIcon('faJsSquare');
      expect(result.library).toBe('material');
      expect(result.icon).toBe('JavascriptOutlined');
    });

    it('should return ionicons5 icon for common icons', () => {
      const result = getViconsIcon('faFolderOpen');
      expect(result.library).toBe('ionicons5');
      expect(result.icon).toBe('FolderOpenOutline');
    });

    it('should prioritize material over ionicons5', () => {
      // If an icon exists in both, material should be preferred
      // Based on the implementation, material is checked first
      const result = getViconsIcon('faJsSquare');
      expect(result.library).toBe('material');
    });

    it('should return default fallback for unknown icons', () => {
      const result = getViconsIcon('faUnknownIcon');
      expect(result.library).toBe('ionicons5');
      expect(result.icon).toBe('HelpOutline');
    });
  });

  describe('faToIonicons5', () => {
    it('should contain file operation icons', () => {
      expect(faToIonicons5).toHaveProperty('faFolderOpen');
      expect(faToIonicons5).toHaveProperty('faFileAlt');
      expect(faToIonicons5).toHaveProperty('faFolder');
    });

    it('should contain action icons', () => {
      expect(faToIonicons5).toHaveProperty('faPlay');
      expect(faToIonicons5).toHaveProperty('faPlus');
      expect(faToIonicons5).toHaveProperty('faCog');
    });

    it('should map to correct Ionicons5 names', () => {
      expect(faToIonicons5.faFolderOpen).toBe('FolderOpenOutline');
      expect(faToIonicons5.faFileAlt).toBe('DocumentOutline');
      expect(faToIonicons5.faPlay).toBe('PlayCircleOutline');
    });
  });

  describe('faToMaterial', () => {
    it('should contain brand icons', () => {
      expect(faToMaterial).toHaveProperty('faJsSquare');
      expect(faToMaterial).toHaveProperty('faPython');
      expect(faToMaterial).toHaveProperty('faHtml5');
    });

    it('should map to correct Material names', () => {
      expect(faToMaterial.faJsSquare).toBe('JavascriptOutlined');
      expect(faToMaterial.faHtml5).toBe('HtmlOutlined');
      expect(faToMaterial.faCss3Alt).toBe('CssOutlined');
    });
  });
});


