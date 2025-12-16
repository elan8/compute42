import { reactive } from 'vue';

export interface HelpContent {
  title: string;
  content: any;
}

export interface HelpSection {
  id: string;
  title: string;
  content: HelpContent | null;
}

class HelpService {
  private helpSections = reactive<HelpSection[]>([{ id: 'about', title: 'About', content: null }]);

  private loadedSections = new Set<string>();

  async loadHelpContent(sectionId: string): Promise<HelpContent | null> {
    if (this.loadedSections.has(sectionId)) {
      const section = this.helpSections.find((s) => s.id === sectionId);
      return section?.content || null;
    }

    // Special handling for About page - no Markdown file needed
    if (sectionId === 'about') {
      const content = {
        title: 'About',
        content: {
          isAboutPage: true,
        },
      };

      const section = this.helpSections.find((s) => s.id === sectionId);
      if (section) {
        section.content = content;
        this.loadedSections.add(sectionId);
      }

      return content;
    }

    try {
      const response = await fetch(`/help/${sectionId}.md`);
      if (!response.ok) {
        console.error(`Failed to load help content for ${sectionId}:`, response.statusText);
        return null;
      }

      const markdownText = await response.text();
      const content = {
        title: this.helpSections.find((s) => s.id === sectionId)?.title || sectionId,
        content: {
          markdown: markdownText,
        },
      };

      const section = this.helpSections.find((s) => s.id === sectionId);
      if (section) {
        section.content = content;
        this.loadedSections.add(sectionId);
      }

      return content;
    } catch (error) {
      console.error(`Error loading help content for ${sectionId}:`, error);
      return null;
    }
  }

  getHelpSections(): HelpSection[] {
    return this.helpSections;
  }

  getHelpSection(sectionId: string): HelpSection | undefined {
    return this.helpSections.find((s) => s.id === sectionId);
  }

  async preloadAllHelpContent(): Promise<void> {
    const loadPromises = this.helpSections.map((section) => this.loadHelpContent(section.id));
    await Promise.all(loadPromises);
  }
}

export const helpService = new HelpService();
