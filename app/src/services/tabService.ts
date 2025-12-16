import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { debug, trace, logError } from '../utils/logger';

// Interface for Tab structure matching the backend
export interface Tab {
  id: string;
  title: string;
  path: string | null;
  content: string;
  is_dirty: boolean;
}

class TabService {
  private tabs: Tab[] = [];
  private activeTabId: string | null = null;
  private listeners: Array<() => void> = [];

  constructor() {
    this.setupEventListeners();
  }

  private async setupEventListeners() {
    try {
      // Listen for state changes from the backend
      const unlistenStateChanged = await listen('state-changed', (event) => {
        const eventType = event.payload as string;
        if (eventType === 'tabs-changed') {
          this.refreshTabs();
        }
      });

      this.listeners.push(unlistenStateChanged);
    } catch (err) {
      await logError('TabService: Failed to setup event listeners', err);
    }
  }

  private async refreshTabs() {
    try {
      const tabs = await invoke<Tab[]>('get_tabs');
      this.tabs = tabs;
    } catch (err) {
      await logError('TabService: Failed to refresh tabs', err);
    }
  }

  async getTabs(): Promise<Tab[]> {
    try {
      this.tabs = await invoke<Tab[]>('get_tabs');
      return this.tabs;
    } catch (err) {
      await logError('TabService: Failed to get tabs', err);
      return [];
    }
  }

  async addTab(tab: Omit<Tab, 'id'>): Promise<string> {
    try {
      const id = `tab_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
      const newTab: Tab = {
        ...tab,
        id,
      };

      await invoke('add_tab', { tab: newTab });
      await this.refreshTabs();

      debug(`TabService: Added tab with id ${id}`);
      return id;
    } catch (err) {
      await logError('TabService: Failed to add tab', err);
      throw err;
    }
  }

  async removeTab(tabId: string): Promise<void> {
    try {
      await invoke('remove_tab', { tabId });
      await this.refreshTabs();

      if (this.activeTabId === tabId) {
        this.activeTabId = this.tabs.length > 0 ? this.tabs[0].id : null;
      }

      debug(`TabService: Removed tab ${tabId}`);
    } catch (err) {
      await logError('TabService: Failed to remove tab', err);
      throw err;
    }
  }

  async updateTab(tabId: string, updatedTab: Tab): Promise<void> {
    try {
      await invoke('update_tab', { tabId, updatedTab });
      await this.refreshTabs();
    } catch (err) {
      await logError('TabService: Failed to update tab', err);
      throw err;
    }
  }

  async updateTabContent(tabId: string, content: string): Promise<void> {
    try {
      await invoke('update_tab_content', { tabId, content });
      await this.refreshTabs();
    } catch (err) {
      await logError('TabService: Failed to update tab content', err);
      throw err;
    }
  }

  async saveTabToFile(tabId: string): Promise<void> {
    try {
      await invoke('save_tab_to_file', { tabId });
      await this.refreshTabs();

      debug(`TabService: Saved tab ${tabId} to file`);
    } catch (err) {
      await logError('TabService: Failed to save tab to file', err);
      throw err;
    }
  }

  async getDirtyTabs(): Promise<Tab[]> {
    try {
      return await invoke<Tab[]>('get_dirty_tabs');
    } catch (err) {
      await logError('TabService: Failed to get dirty tabs', err);
      return [];
    }
  }

  async clearTabs(): Promise<void> {
    try {
      await invoke('clear_tabs');
      this.tabs = [];
      this.activeTabId = null;

      debug('TabService: Cleared all tabs');
    } catch (err) {
      await logError('TabService: Failed to clear tabs', err);
      throw err;
    }
  }

  setActiveTab(tabId: string | null) {
    this.activeTabId = tabId;
  }

  getActiveTabId(): string | null {
    return this.activeTabId;
  }

  getTabById(tabId: string): Tab | undefined {
    return this.tabs.find((tab) => tab.id === tabId);
  }

  getTabByPath(path: string): Tab | undefined {
    return this.tabs.find((tab) => tab.path === path);
  }

  cleanup() {
    this.listeners.forEach((unlisten) => unlisten());
    this.listeners = [];
  }
}

export const tabService = new TabService();
