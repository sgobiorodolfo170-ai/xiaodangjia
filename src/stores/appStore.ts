import { create } from 'zustand';
import { FileNode, Project, OpenTab, Viewport, FileRelation } from '../types';

interface AppState {
  projects: Project[];
  currentProject: Project | null;
  setProjects: (projects: Project[]) => void;
  setCurrentProject: (project: Project | null) => void;
  
  fileNodes: FileNode[];
  setFileNodes: (nodes: FileNode[]) => void;
  addFileNode: (node: FileNode) => void;
  updateFileNode: (id: string, updates: Partial<FileNode>) => void;
  removeFileNode: (id: string) => void;
  
  relations: FileRelation[];
  setRelations: (relations: FileRelation[]) => void;
  
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
    fileNodes: state.fileNodes.filter((n) => n.id !== id)
  })),
  
  relations: [],
  setRelations: (relations) => set({ relations }),
  
  openTabs: [],
  activeTabId: null,
  openTab: (tab) => set((state) => ({
    openTabs: [...state.openTabs, tab],
    activeTabId: tab.id
  })),
  closeTab: (id) => set((state) => {
    const newTabs = state.openTabs.filter((t) => t.id !== id);
    let newActiveId = state.activeTabId;
    if (state.activeTabId === id) {
      const idx = state.openTabs.findIndex((t) => t.id === id);
      newActiveId = newTabs[idx]?.id ?? newTabs[idx - 1]?.id ?? null;
    }
    return { openTabs: newTabs, activeTabId: newActiveId };
  }),
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
  
  isLoading: false,
  setIsLoading: (loading) => set({ isLoading: loading }),
  
  updateNodePosition: (id, x, y) => {
    const { fileNodes } = get();
    set({
      fileNodes: fileNodes.map((n) => 
        n.id === id ? { ...n, positionX: x, positionY: y } : n
      )
    });
  },
}));
