// 项目类型
export interface Project {
  id: string;
  name: string;
  rootPath: string;
  createdAt: string;
  updatedAt: string;
}

// 文件节点类型
export interface FileNode {
  id: string;
  projectId: string;
  path: string;
  name: string;
  extension: string;
  size: number;
  createdAt: string | null;
  modifiedAt: string | null;
  tags: string[];
  parentId: string | null;
  positionX: number;
  positionY: number;
  isCollapsed: boolean;
  isDirectory: boolean;
  children: string[];
  relatedFiles: string[];
}

export interface FileRelation {
  id: string;
  projectId: string;
  sourceId: string;
  targetId: string;
  relationType: 'similar' | 'import' | 'reference' | 'auto';
  confidence: number;
}

export interface FileContent {
  path: string;
  content: string;
  encoding: string;
  size: number;
}

export interface OpenTab {
  id: string;
  fileId: string;
  path: string;
  name: string;
  type: 'viewer' | 'editor';
  isModified: boolean;
  content?: string;  // persisted editor content so switching tabs doesn't lose edits
}

export interface Viewport {
  x: number;
  y: number;
  zoom: number;
}

export interface SimilarityResult {
  id: string;
  score: number;
}

export interface ArchiveSuggestion {
  directory: string;
  confidence: number;
  reason: string;
}

// Note: plugin_system was removed as dead code.
// If plugin support is reintroduced, add PluginMetadata here.
