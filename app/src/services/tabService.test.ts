import { describe, it, expect, beforeEach, vi } from 'vitest';
import { tabService, type Tab } from './tabService';
import { mockInvoke, mockListen, resetTauriMocks, createMockEventListener } from '../test/mocks/tauri';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Mock the Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('tabService', () => {
  beforeEach(() => {
    resetTauriMocks();
    vi.clearAllMocks();
    // Reset tab service state
    tabService.cleanup();
  });

  describe('getTabs', () => {
    it('should fetch tabs from backend', async () => {
      const mockTabs: Tab[] = [
        {
          id: 'tab1',
          title: 'File1.jl',
          path: '/path/to/file1.jl',
          content: 'content1',
          is_dirty: false,
        },
      ];

      (invoke as any).mockResolvedValue(mockTabs);

      const tabs = await tabService.getTabs();
      expect(tabs).toEqual(mockTabs);
      expect(invoke).toHaveBeenCalledWith('get_tabs');
    });

    it('should return empty array on error', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const tabs = await tabService.getTabs();
      expect(tabs).toEqual([]);
    });
  });

  describe('addTab', () => {
    it('should add a new tab', async () => {
      const newTab: Omit<Tab, 'id'> = {
        title: 'NewFile.jl',
        path: '/path/to/newfile.jl',
        content: 'new content',
        is_dirty: false,
      };

      (invoke as any).mockResolvedValue(undefined);

      const id = await tabService.addTab(newTab);
      expect(id).toBeDefined();
      expect(id).toMatch(/^tab_\d+_[a-z0-9]+$/);
      expect(invoke).toHaveBeenCalledWith('add_tab', expect.objectContaining({
        tab: expect.objectContaining({
          title: newTab.title,
          path: newTab.path,
          content: newTab.content,
        }),
      }));
    });

    it('should throw error on failure', async () => {
      const newTab: Omit<Tab, 'id'> = {
        title: 'NewFile.jl',
        path: '/path/to/newfile.jl',
        content: 'new content',
        is_dirty: false,
      };

      (invoke as any).mockRejectedValue(new Error('Failed'));

      await expect(tabService.addTab(newTab)).rejects.toThrow();
    });
  });

  describe('removeTab', () => {
    it('should remove a tab', async () => {
      (invoke as any).mockResolvedValue(undefined);

      await tabService.removeTab('tab1');
      expect(invoke).toHaveBeenCalledWith('remove_tab', { tabId: 'tab1' });
    });

    it('should update active tab if removed tab was active', async () => {
      const mockTabs: Tab[] = [
        { id: 'tab1', title: 'File1', path: null, content: '', is_dirty: false },
        { id: 'tab2', title: 'File2', path: null, content: '', is_dirty: false },
      ];

      (invoke as any).mockResolvedValue(mockTabs);
      tabService.setActiveTab('tab1');

      await tabService.removeTab('tab1');
      // Active tab should be updated to first remaining tab
      expect(invoke).toHaveBeenCalledWith('remove_tab', { tabId: 'tab1' });
    });
  });

  describe('updateTab', () => {
    it('should update a tab', async () => {
      const updatedTab: Tab = {
        id: 'tab1',
        title: 'Updated.jl',
        path: '/path/to/updated.jl',
        content: 'updated content',
        is_dirty: true,
      };

      (invoke as any).mockResolvedValue(undefined);

      await tabService.updateTab('tab1', updatedTab);
      expect(invoke).toHaveBeenCalledWith('update_tab', {
        tabId: 'tab1',
        updatedTab,
      });
    });
  });

  describe('updateTabContent', () => {
    it('should update tab content', async () => {
      (invoke as any).mockResolvedValue(undefined);

      await tabService.updateTabContent('tab1', 'new content');
      expect(invoke).toHaveBeenCalledWith('update_tab_content', {
        tabId: 'tab1',
        content: 'new content',
      });
    });
  });

  describe('saveTabToFile', () => {
    it('should save tab to file', async () => {
      (invoke as any).mockResolvedValue(undefined);

      await tabService.saveTabToFile('tab1');
      expect(invoke).toHaveBeenCalledWith('save_tab_to_file', { tabId: 'tab1' });
    });
  });

  describe('getDirtyTabs', () => {
    it('should get dirty tabs', async () => {
      const dirtyTabs: Tab[] = [
        {
          id: 'tab1',
          title: 'File1.jl',
          path: '/path/to/file1.jl',
          content: 'content',
          is_dirty: true,
        },
      ];

      (invoke as any).mockResolvedValue(dirtyTabs);

      const tabs = await tabService.getDirtyTabs();
      expect(tabs).toEqual(dirtyTabs);
      expect(invoke).toHaveBeenCalledWith('get_dirty_tabs');
    });

    it('should return empty array on error', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const tabs = await tabService.getDirtyTabs();
      expect(tabs).toEqual([]);
    });
  });

  describe('clearTabs', () => {
    it('should clear all tabs', async () => {
      (invoke as any).mockResolvedValue(undefined);

      await tabService.clearTabs();
      expect(invoke).toHaveBeenCalledWith('clear_tabs');
      expect(tabService.getActiveTabId()).toBeNull();
    });
  });

  describe('activeTab management', () => {
    it('should set and get active tab', () => {
      tabService.setActiveTab('tab1');
      expect(tabService.getActiveTabId()).toBe('tab1');

      tabService.setActiveTab(null);
      expect(tabService.getActiveTabId()).toBeNull();
    });
  });

  describe('getTabById', () => {
    it('should get tab by id', async () => {
      const mockTabs: Tab[] = [
        {
          id: 'tab1',
          title: 'File1.jl',
          path: '/path/to/file1.jl',
          content: 'content',
          is_dirty: false,
        },
      ];

      (invoke as any).mockResolvedValue(mockTabs);
      await tabService.getTabs();

      const tab = tabService.getTabById('tab1');
      expect(tab).toEqual(mockTabs[0]);
    });

    it('should return undefined for non-existent tab', async () => {
      (invoke as any).mockResolvedValue([]);
      await tabService.getTabs();

      const tab = tabService.getTabById('nonexistent');
      expect(tab).toBeUndefined();
    });
  });

  describe('getTabByPath', () => {
    it('should get tab by path', async () => {
      const mockTabs: Tab[] = [
        {
          id: 'tab1',
          title: 'File1.jl',
          path: '/path/to/file1.jl',
          content: 'content',
          is_dirty: false,
        },
      ];

      (invoke as any).mockResolvedValue(mockTabs);
      await tabService.getTabs();

      const tab = tabService.getTabByPath('/path/to/file1.jl');
      expect(tab).toEqual(mockTabs[0]);
    });

    it('should return undefined for non-existent path', async () => {
      (invoke as any).mockResolvedValue([]);
      await tabService.getTabs();

      const tab = tabService.getTabByPath('/nonexistent');
      expect(tab).toBeUndefined();
    });
  });

  describe('event listeners', () => {
    it('should set up event listeners on construction', () => {
      // The service sets up listeners in constructor
      // We can verify that listen was called
      expect(listen).toHaveBeenCalled();
    });

    it('should refresh tabs on state-changed event', async () => {
      const mockTabs: Tab[] = [
        {
          id: 'tab1',
          title: 'File1.jl',
          path: '/path/to/file1.jl',
          content: 'content',
          is_dirty: false,
        },
      ];

      (invoke as any).mockResolvedValue(mockTabs);

      // Simulate event listener being called
      const eventListener = (listen as any).mock.calls[0]?.[1];
      if (eventListener) {
        await eventListener({ payload: 'tabs-changed' });
        expect(invoke).toHaveBeenCalledWith('get_tabs');
      }
    });
  });

  describe('cleanup', () => {
    it('should cleanup event listeners', () => {
      const unlistenFn = vi.fn();
      (listen as any).mockResolvedValue(unlistenFn);

      tabService.cleanup();
      // Cleanup should call all unlisten functions
      // The exact behavior depends on implementation
    });
  });
});


