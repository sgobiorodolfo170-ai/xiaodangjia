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
  createdAt: string;
  modifiedAt: string;
  tags: string[];
  parentId: string | null;
  positionX: number;
  positionY: number;
  isCollapsed: boolean;
  isDirectory: boolean;
  children: string[];
  relatedFiles: string[];
}

// 文件关联
export interface FileRelation {
  id: string;
  projectId: string;
  sourceId: string;
  targetId: string;
  relationType: 'similar' | 'import' | 'reference' | 'auto';
  confidence: number;
}

// 文件内容
export interface FileContent {
  path: string;
  content: string;
  encoding: string;
  size: number;
}

// 打开的标签页
export interface OpenTab {
  id: string;
  fileId: string;
  path: string;
  name: string;
  type: 'viewer' | 'editor';
  isModified: boolean;
}

// 画布视口
export interface Viewport {
  x: number;
  y: number;
  zoom: number;
}
