import { create } from 'zustand';
import { FileNode, Project, OpenTab, Viewport, FileRelation } from '../types';
import { Tag } from '../services/api';

interface AppState {
  projects: Project[];
  currentProject: Project | null;
  setProjects: (projects: Project[]) => void;
  setCurrentProject: (project: Project | null) => void;

  fileNodes: FileNode[];
  setFileNodes: (nodes: FileNode[]) => void;
  addFileNode: (node: FileNode) => void;
  updateFileNode: (id: string, updates: Partial<FileNode>) => void;
  updateFileTags: (id: string, tags: string[]) => void;
  removeFileNode: (id: string) => void;
  
  // Favorites state
  favoriteIds: string[];
  setFavoriteIds: (ids: string[]) => void;
  addFavoriteId: (id: string) => void;
  removeFavoriteId: (id: string) => void;
  
  // Custom tags state
  customTags: Tag[];
  setCustomTags: (tags: Tag[]) => void;
  addCustomTag: (tag: Tag) => void;
  removeCustomTag: (id: string) => void;
  
  // File custom tags mapping
  fileCustomTags: Record<string, Tag[]>;
  setFileCustomTags: (fileId: string, tags: Tag[]) => void;
  addFileCustomTag: (fileId: string, tag: Tag) => void;
  removeFileCustomTag: (fileId: string, tagId: string) => void;
  
  relations: FileRelation[];
  setRelations: (relations: FileRelation[]) => void;
  clearProjectState: () => void;
  
  openTabs: OpenTab[];
  activeTabId: string | null;
  openTab: (tab: OpenTab) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string | null) => void;
  updateTab: (id: string, updates: Partial<OpenTab>) => void;
  
  viewport: Viewport;
  setViewport: (viewport: Viewport) => void;
  
  selectedNodeIds: string[];
  setSelectedNodeIds: (ids: string[]) => void;
  
  isLoading: boolean;
  setIsLoading: (loading: boolean) => void;
  loadingCount: number;
  startLoading: () => void;
  stopLoading: () => void;
  
  darkMode: boolean;
  setDarkMode: (dark: boolean) => void;
  updateNodePosition: (id: string, x: number, y: number) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  projects: [],
  currentProject: null,
  setProjects: (projects) => set({ projects }),
  setCurrentProject: (project) => set({ currentProject: project }),
  
  fileNodes: [],
  setFileNodes: (nodes) => set({ fileNodes: nodes }),
  addFileNode: (node) => set((state) => ({ 
    fileNodes: [...state.fileNodes, node] 
  })),
  updateFileNode: (id, updates) => set((state) => ({
    fileNodes: state.fileNodes.map((n) => 
      n.id === id ? { ...n, ...updates } : n
    )
  })),
  removeFileNode: (id) => set((state) => ({
    fileNodes: state.fileNodes.filter((n) => n.id !== id),
    // Cascade: remove from favoriteIds when node is deleted
    favoriteIds: state.favoriteIds.filter((fid) => fid !== id),
  })),

  updateFileTags: (id, tags) => set((state) => ({
    fileNodes: state.fileNodes.map((n) =>
      n.id === id ? { ...n, tags } : n
    )
  })),

  // Favorites
  favoriteIds: [],
  setFavoriteIds: (ids) => set({ favoriteIds: ids }),
  addFavoriteId: (id) => set((state) => ({ 
    favoriteIds: [...state.favoriteIds, id] 
  })),
  removeFavoriteId: (id) => set((state) => ({ 
    favoriteIds: state.favoriteIds.filter((fid) => fid !== id) 
  })),
  
  // Custom tags
  customTags: [],
  setCustomTags: (tags) => set({ customTags: tags }),
  addCustomTag: (tag) => set((state) => ({ 
    customTags: [...state.customTags, tag] 
  })),
  removeCustomTag: (id) => set((state) => ({ 
    customTags: state.customTags.filter((t) => t.id !== id) 
  })),
  
  // File custom tags — changes also sync into FileNode.tags for unified display
  fileCustomTags: {},
  setFileCustomTags: (fileId, tags) => set((state) => {
    const customTagNames = tags.map((t) => t.name);
    const node = state.fileNodes.find((n) => n.id === fileId);
    // Merge: auto-generated tags (not in customTags list) + new custom tag names
    const autoTags = node ? node.tags.filter((t) => !state.customTags.some((ct) => ct.name === t) && !customTagNames.includes(t)) : [];
    const mergedTags = [...autoTags, ...customTagNames];
    return {
      fileCustomTags: { ...state.fileCustomTags, [fileId]: tags },
      fileNodes: state.fileNodes.map((n) =>
        n.id === fileId ? { ...n, tags: mergedTags } : n
      ),
    };
  }),
  addFileCustomTag: (fileId, tag) => set((state) => { 
    const existing = state.fileCustomTags[fileId] || [];
    const node = state.fileNodes.find((n) => n.id === fileId);
    const tags = node ? node.tags : [];
    return { 
      fileCustomTags: { 
        ...state.fileCustomTags, 
        [fileId]: [...existing, tag] 
      },
      fileNodes: state.fileNodes.map((n) =>
        n.id === fileId && !n.tags.includes(tag.name) ? { ...n, tags: [...n.tags, tag.name] } : n
      ),
    };
  }),
  removeFileCustomTag: (fileId, tagId) => set((state) => {
    const existing = state.fileCustomTags[fileId] || [];
    const removedTag = existing.find((t) => t.id === tagId);
    return { 
      fileCustomTags: { 
        ...state.fileCustomTags, 
        [fileId]: existing.filter((t) => t.id !== tagId) 
      },
      fileNodes: state.fileNodes.map((n) =>
        n.id === fileId && removedTag ? { ...n, tags: n.tags.filter((t) => t !== removedTag.name) } : n
      ),
    };
  }),

  relations: [],
  setRelations: (relations) => set({ relations }),
  clearProjectState: () => set({
    openTabs: [],
    activeTabId: null,
    selectedNodeIds: [],
    relations: [],
    favoriteIds: [],
    fileCustomTags: {},
  }),
  
  openTabs: [],
  activeTabId: null,
  openTab: (tab) => set((state) => {
    // Prevent duplicate tabs - reuse existing tab if fileId already open
    const existing = state.openTabs.find((t) => t.fileId === tab.fileId);
    if (existing) {
      return { activeTabId: existing.id };
    }
    return {
      openTabs: [...state.openTabs, tab],
      activeTabId: tab.id,
    };
  }),
  closeTab: (id) => {
    const state = get();
    const tab = state.openTabs.find((t) => t.id === id);
    if (tab?.isModified && !confirm(`"${tab.name}" 有未保存的更改，确定要关闭吗？`)) {
      return;
    }
    set((state) => {
      const newTabs = state.openTabs.filter((t) => t.id !== id);
      let newActiveId = state.activeTabId;
      if (state.activeTabId === id) {
        const idx = state.openTabs.findIndex((t) => t.id === id);
        newActiveId = newTabs[idx]?.id ?? newTabs[idx - 1]?.id ?? null;
      }
      return { openTabs: newTabs, activeTabId: newActiveId };
    });
  },
  setActiveTab: (id) => set({ activeTabId: id }),
  updateTab: (id, updates) => set((state) => ({
    openTabs: state.openTabs.map((t) => 
      t.id === id ? { ...t, ...updates } : t
    )
  })),
  
  viewport: { x: 0, y: 0, zoom: 1 },
  setViewport: (viewport) => set({ viewport }),
  
  selectedNodeIds: [],
  setSelectedNodeIds: (ids) => set({ selectedNodeIds: ids }),
  
  darkMode: false,
  setDarkMode: (dark) => set({ darkMode: dark }),
  isLoading: false,
  setIsLoading: (loading) => set({ isLoading: loading }),
  loadingCount: 0,
  startLoading: () => set((state) => ({ loadingCount: state.loadingCount + 1, isLoading: true })),
  stopLoading: () => set((state) => {
    const newCount = Math.max(0, state.loadingCount - 1);
    return { loadingCount: newCount, isLoading: newCount > 0 };
  }),
  
  updateNodePosition: (id, x, y) => {
    const { fileNodes } = get();
    set({
      fileNodes: fileNodes.map((n) => 
        n.id === id ? { ...n, positionX: x, positionY: y } : n
      )
    });
  },
}));
