import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useAppStore } from './appStore';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('appStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe('projectPath', () => {
    it('should set and get project path', async () => {
      const store = useAppStore();
      await store.setProjectPath('/path/to/project');

      expect(store.projectPath).toBe('/path/to/project');
    });

    it('should set project path to null', async () => {
      const store = useAppStore();
      await store.setProjectPath('/path/to/project');
      await store.setProjectPath(null);

      expect(store.projectPath).toBeNull();
    });

    it('should get active project path', () => {
      const store = useAppStore();
      store.setProjectPath('/path/to/project');

      expect(store.getActiveProjectPath()).toBe('/path/to/project');
    });
  });

  describe('fileToOpen', () => {
    it('should set and get file to open', () => {
      const store = useAppStore();
      store.setFileToOpen('/path/to/file.jl');

      expect(store.fileToOpen).toBe('/path/to/file.jl');
    });
  });

  describe('isJuliaProject', () => {
    it('should set and get Julia project status', () => {
      const store = useAppStore();
      store.setIsJuliaProject(true);

      expect(store.isJuliaProject).toBe(true);
    });

    it('should check if Julia project is active', () => {
      const store = useAppStore();
      store.setProjectPath('/path/to/project');
      store.setIsJuliaProject(true);

      expect(store.isJuliaProjectActive()).toBe(true);
    });

    it('should return false if no project path', () => {
      const store = useAppStore();
      store.setIsJuliaProject(true);

      expect(store.isJuliaProjectActive()).toBe(false);
    });
  });

  describe('backendBusy', () => {
    it('should set and get backend busy status', () => {
      const store = useAppStore();
      store.setBackendBusy(true);

      expect(store.backendBusy).toBe(true);
      expect(store.getBackendBusyStatus()).toBe(true);
    });

    it('should sync backend busy status', async () => {
      (invoke as any).mockResolvedValue(true);

      const store = useAppStore();
      await store.syncBackendBusyStatus();

      expect(invoke).toHaveBeenCalledWith('get_backend_busy_status');
      expect(store.backendBusy).toBe(true);
    });

    it('should force sync backend busy status', async () => {
      (invoke as any).mockResolvedValue(false);

      const store = useAppStore();
      await store.forceSyncBackendBusyStatus();

      expect(invoke).toHaveBeenCalledWith('get_backend_busy_status');
      expect(store.backendBusy).toBe(false);
    });
  });

  describe('LSP status', () => {
    it('should set LSP status', () => {
      const store = useAppStore();
      store.setLspStatus({
        status: 'ready',
        message: 'LSP is ready',
      });

      expect(store.lspStatus.status).toBe('ready');
      expect(store.lspStatus.message).toBe('LSP is ready');
    });

    it('should update LSP status partially', () => {
      const store = useAppStore();
      store.setLspStatus({
        status: 'starting',
        message: 'Starting...',
      });

      store.updateLspStatus({
        status: 'ready',
        message: 'Ready',
      });

      expect(store.lspStatus.status).toBe('ready');
      expect(store.lspStatus.message).toBe('Ready');
    });
  });

  describe('workspaceVariables', () => {
    it('should set workspace variables', () => {
      const store = useAppStore();
      store.setWorkspaceVariables({
        x: 1,
        y: 'test',
      });

      expect(store.workspaceVariables).toEqual({
        x: 1,
        y: 'test',
      });
    });
  });

  describe('openFiles and tabs', () => {
    it('should add open file', () => {
      const store = useAppStore();
      store.addOpenFile({
        path: '/path/to/file.jl',
        name: 'file.jl',
        content: 'content',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: false,
        viewerType: 'monaco',
      });

      expect(store.openFiles.length).toBe(1);
      expect(store.openFiles[0].path).toBe('/path/to/file.jl');
    });

    it('should update existing open file', () => {
      const store = useAppStore();
      store.addOpenFile({
        path: '/path/to/file.jl',
        name: 'file.jl',
        content: 'content',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: false,
        viewerType: 'monaco',
      });

      store.addOpenFile({
        path: '/path/to/file.jl',
        name: 'file.jl',
        content: 'updated content',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: true,
        viewerType: 'monaco',
      });

      expect(store.openFiles.length).toBe(1);
      expect(store.openFiles[0].content).toBe('updated content');
      expect(store.openFiles[0].isDirty).toBe(true);
    });

    it('should remove open file', () => {
      const store = useAppStore();
      store.addOpenFile({
        path: '/path/to/file1.jl',
        name: 'file1.jl',
        content: 'content1',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: false,
        viewerType: 'monaco',
      });

      store.addOpenFile({
        path: '/path/to/file2.jl',
        name: 'file2.jl',
        content: 'content2',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: false,
        viewerType: 'monaco',
      });

      store.setActiveTab('/path/to/file1.jl');
      store.removeOpenFile('/path/to/file1.jl');

      expect(store.openFiles.length).toBe(1);
      expect(store.openFiles[0].path).toBe('/path/to/file2.jl');
      // Active tab should switch to remaining file
      expect(store.activeTab).toBe('/path/to/file2.jl');
    });

    it('should set active tab', () => {
      const store = useAppStore();
      store.setActiveTab('/path/to/file.jl');

      expect(store.activeTab).toBe('/path/to/file.jl');
    });

    it('should clear all tabs', () => {
      const store = useAppStore();
      store.addOpenFile({
        path: '/path/to/file1.jl',
        name: 'file1.jl',
        content: 'content',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: false,
        viewerType: 'monaco',
      });

      store.clearAllTabs();

      expect(store.openFiles.length).toBe(0);
      expect(store.activeTab).toBeNull();
    });

    it('should update open file', () => {
      const store = useAppStore();
      store.addOpenFile({
        path: '/path/to/file.jl',
        name: 'file.jl',
        content: 'content',
        language: 'julia',
        loading: false,
        error: false,
        readOnly: false,
        isDirty: false,
        viewerType: 'monaco',
      });

      store.updateOpenFile('/path/to/file.jl', {
        isDirty: true,
        content: 'updated',
      });

      expect(store.openFiles[0].isDirty).toBe(true);
      expect(store.openFiles[0].content).toBe('updated');
    });
  });
});


