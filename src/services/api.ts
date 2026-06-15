import { invoke } from '@tauri-apps/api/core';
import { FileNode, Project, FileContent, FileRelation } from '../types';

export async function createProject(name: string, rootPath: string): Promise<Project> {
  return invoke('create_project', { name, rootPath });
}

export async function listProjects(): Promise<Project[]> {
  return invoke('list_projects');
}

export async function getProject(id: string): Promise<Project> {
  return invoke('get_project', { id });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke('delete_project', { id });
}

export async function scanDirectory(projectId: string, path: string): Promise<FileNode[]> {
  return invoke('scan_directory', { projectId, path });
}

export async function readFileContent(path: string): Promise<FileContent> {
  return invoke('read_file_content', { path });
}

export async function writeFileContent(path: string, content: string): Promise<void> {
  return invoke('write_file_content', { path, content });
}

export async function deleteFile(projectId: string, path: string): Promise<void> {
  return invoke('delete_file', { projectId, path });
}

export async function renameFile(projectId: string, oldPath: string, newPath: string): Promise<void> {
  return invoke('rename_file', { projectId, oldPath, newPath });
}

export async function updateNodePosition(id: string, x: number, y: number): Promise<void> {
  return invoke('update_node_position', { id, x, y });
}

export async function analyzeFileRelations(projectId: string): Promise<FileRelation[]> {
  return invoke('analyze_file_relations', { projectId });
}

export async function generateTags(projectId: string, fileId: string): Promise<string[]> {
  return invoke('generate_tags', { projectId, fileId });
}

export async function searchFiles(projectId: string, query: string): Promise<FileNode[]> {
  return invoke('search_files', { projectId, query });
}

export async function findSimilarFiles(projectId: string, fileId: string): Promise<FileNode[]> {
  return invoke('find_similar_files', { projectId, fileId });
}

export async function openDirectoryDialog(): Promise<string | null> {
  return invoke('open_directory_dialog');
}

export async function startFileWatcher(projectId: string, path: string): Promise<void> {
  return invoke('start_file_watcher', { projectId, path });
}
