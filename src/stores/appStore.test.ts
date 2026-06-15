
import { describe, it, expect, beforeEach } from 'vitest';
import { useAppStore } from './appStore';
import type { FileNode, Project, OpenTab } from '../types';

function resetStore() {
  useAppStore.setState({
    projects: [],
    currentProject: null,
    fileNodes: [],
    relations: [],
    openTabs: [],
    activeTabId: null,
    viewport: { x: 0, y: 0, zoom: 1 },
    selectedNodeIds: [],
    isLoading: false,
    darkMode: false,
  });
}

const mockProject: Project = {
  id: 'proj-1', name: 'Test Project', rootPath: '/tmp/test',
  createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-01-01T00:00:00Z',
};

const mockFileNode: FileNode = {
  id: 'file-1', projectId: 'proj-1', path: '/tmp/test/main.ts',
  name: 'main.ts', extension: 'ts', size: 1024,
  createdAt: '2026-01-01T00:00:00Z', modifiedAt: '2026-01-01T00:00:00Z',
  tags: ['TypeScript'], parentId: null,
  positionX: 0, positionY: 0, isCollapsed: false, isDirectory: false,
  children: [], relatedFiles: [],
};

const mockTab: OpenTab = {
  id: 'tab-1', fileId: 'file-1', path: '/tmp/test/main.ts',
  name: 'main.ts', type: 'editor', isModified: false,
};

describe('appStore', () => {
  beforeEach(() => { resetStore(); });

  describe('projects', () => {
    it('starts empty', () => {
      expect(useAppStore.getState().projects).toEqual([]);
    });
    it('sets projects', () => {
      useAppStore.getState().setProjects([mockProject]);
      expect(useAppStore.getState().projects).toHaveLength(1);
    });
    it('sets current project', () => {
      useAppStore.getState().setCurrentProject(mockProject);
      expect(useAppStore.getState().currentProject?.name).toBe('Test Project');
    });
    it('clears current project', () => {
      useAppStore.getState().setCurrentProject(mockProject);
      useAppStore.getState().setCurrentProject(null);
      expect(useAppStore.getState().currentProject).toBeNull();
    });
  });

  describe('fileNodes', () => {
    it('adds and removes nodes', () => {
      useAppStore.getState().addFileNode(mockFileNode);
      expect(useAppStore.getState().fileNodes).toHaveLength(1);
      useAppStore.getState().removeFileNode('file-1');
      expect(useAppStore.getState().fileNodes).toHaveLength(0);
    });
    it('updates a node', () => {
      useAppStore.getState().addFileNode(mockFileNode);
      useAppStore.getState().updateFileNode('file-1', { size: 2048 });
      expect(useAppStore.getState().fileNodes[0].size).toBe(2048);
    });
    it('updates node position', () => {
      useAppStore.getState().addFileNode(mockFileNode);
      useAppStore.getState().updateNodePosition('file-1', 100, 200);
      expect(useAppStore.getState().fileNodes[0].positionX).toBe(100);
      expect(useAppStore.getState().fileNodes[0].positionY).toBe(200);
    });
  });

  describe('tabs', () => {
    it('opens a tab', () => {
      useAppStore.getState().openTab(mockTab);
      expect(useAppStore.getState().openTabs).toHaveLength(1);
      expect(useAppStore.getState().activeTabId).toBe('tab-1');
    });
    it('prevents duplicate tabs by fileId', () => {
      useAppStore.getState().openTab(mockTab);
      useAppStore.getState().openTab({ ...mockTab, id: 'tab-dup' });
      expect(useAppStore.getState().openTabs).toHaveLength(1);
    });
    it('closes a tab and switches active', () => {
      useAppStore.getState().openTab(mockTab);
      useAppStore.getState().openTab({ id: 'tab-2', fileId: 'file-2', path: '/tmp/test/bar.ts', name: 'bar.ts', type: 'editor', isModified: false });
      useAppStore.getState().closeTab('tab-1');
      expect(useAppStore.getState().openTabs).toHaveLength(1);
      expect(useAppStore.getState().activeTabId).toBe('tab-2');
    });
    it('updates a tab', () => {
      useAppStore.getState().openTab(mockTab);
      useAppStore.getState().updateTab('tab-1', { isModified: true });
      expect(useAppStore.getState().openTabs[0].isModified).toBe(true);
    });
  });

  describe('viewport', () => {
    it('sets viewport', () => {
      useAppStore.getState().setViewport({ x: 100, y: 200, zoom: 2 });
      expect(useAppStore.getState().viewport).toEqual({ x: 100, y: 200, zoom: 2 });
    });
  });

  describe('UI state', () => {
    it('toggles dark mode', () => {
      useAppStore.getState().setDarkMode(true);
      expect(useAppStore.getState().darkMode).toBe(true);
    });
    it('sets loading', () => {
      useAppStore.getState().setIsLoading(true);
      expect(useAppStore.getState().isLoading).toBe(true);
    });
  });
});
