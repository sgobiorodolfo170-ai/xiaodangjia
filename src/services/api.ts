import { invoke } from '@tauri-apps/api/core';
import { FileNode, Project, FileContent, FileRelation, SimilarityResult, ArchiveSuggestion } from '../types';

/** Unified error wrapper for all Tauri invoke calls (P1-14).
 *  Converts Rust-side errors into friendly messages with an error code. */
class AppError extends Error {
  code: string;
  constructor(message: string, code: string = 'UNKNOWN') {
    super(message);
    this.name = 'AppError';
    this.code = code;
  }
}

async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e: unknown) {
    if (e instanceof Error) {
      throw new AppError(e.message);
    }
    const msg = typeof e === 'string' ? e : String(e);
    throw new AppError(msg);
  }
}



export async function createProject(name: string, rootPath: string): Promise<Project> {
  return safeInvoke('create_project', { name, rootPath });
}

export async function listProjects(): Promise<Project[]> {
  return safeInvoke('list_projects');
}

export async function getProject(id: string): Promise<Project> {
  return safeInvoke('get_project', { id });
}

export async function deleteProject(id: string): Promise<void> {
  return safeInvoke('delete_project', { id });
}

export async function scanDirectory(projectId: string, path: string): Promise<FileNode[]> {
  return safeInvoke('scan_directory', { projectId, path });
}

export async function readFileContent(path: string, projectId?: string): Promise<FileContent> {
  return safeInvoke('read_file_content', { path, projectId: projectId || null });
}

export async function writeFileContent(path: string, content: string, fileId?: string, projectId?: string): Promise<void> {
  return safeInvoke('write_file_content', { path, content, fileId: fileId || null, projectId: projectId || null });
}

// ============= File Edit History API =============

export interface FileEditHistory {
  id: string;
  fileId: string;
  filePath: string;
  content: string;
  diff: string | null;
  createdAt: string;
}

export async function getFileHistory(fileId: string): Promise<FileEditHistory[]> {
  return safeInvoke('get_file_history', { fileId });
}

export async function restoreFileVersion(versionId: string): Promise<string> {
  return safeInvoke('restore_file_version', { versionId });
}

export async function deleteFileHistoryVersion(versionId: string): Promise<void> {
  return safeInvoke('delete_file_history_version', { versionId });
}

export async function deleteFile(projectId: string, path: string): Promise<void> {
  return safeInvoke('delete_file', { projectId, path });
}

export async function renameFile(projectId: string, oldPath: string, newPath: string): Promise<void> {
  return safeInvoke('rename_file', { projectId, oldPath, newPath });
}

export async function updateNodePosition(id: string, x: number, y: number): Promise<void> {
  return safeInvoke('update_node_position', { id, x, y });
}

export async function analyzeFileRelations(projectId: string): Promise<FileRelation[]> {
  return safeInvoke('analyze_file_relations', { projectId });
}

export async function generateTags(projectId: string, fileId: string): Promise<string[]> {
  return safeInvoke('generate_tags', { projectId, fileId });
}

export async function searchFiles(projectId: string, query: string): Promise<FileNode[]> {
  return safeInvoke('search_files', { projectId, query });
}

export async function findSimilarFiles(projectId: string, fileId: string): Promise<FileNode[]> {
  return safeInvoke('find_similar_files', { projectId, fileId });
}

export async function semanticSearchFiles(projectId: string, query: string): Promise<FileNode[]> {
  return safeInvoke('semantic_search_files', { projectId, query });
}

export async function findSimilarByContent(projectId: string, fileId: string, topK?: number): Promise<SimilarityResult[]> {
  return safeInvoke('find_similar_by_content', { projectId, fileId, topK: topK ?? null });
}

export async function findSimilarByEmbedding(projectId: string, fileId: string, topK?: number): Promise<SimilarityResult[]> {
  return safeInvoke('find_similar_by_embedding', { projectId, fileId, topK: topK ?? null });
}

export async function suggestArchiveLocation(projectId: string, fileId: string): Promise<ArchiveSuggestion[]> {
  return safeInvoke('suggest_archive_location', { projectId, fileId });
}

export async function parseFileImports(path: string): Promise<string[]> {
  return safeInvoke('parse_file_imports', { path });
}

export async function analyzeImportRelations(projectId: string): Promise<ImportRelationResult[]> {
  return safeInvoke('analyze_import_relations', { projectId });
}

export async function openDirectoryDialog(): Promise<string | null> {
  return safeInvoke('open_directory_dialog');
}

export async function createDirectory(path: string, projectId?: string): Promise<void> {
  return safeInvoke('create_directory', { path, projectId: projectId || null });
}

export async function moveFile(source: string, destination: string, projectId?: string): Promise<void> {
  return safeInvoke('move_file', { source, destination, projectId: projectId || null });
}

export async function copyFile(source: string, destination: string, projectId?: string): Promise<void> {
  return safeInvoke('copy_file', { source, destination, projectId: projectId || null });
}

export async function trashFile(path: string, projectId?: string): Promise<void> {
  return safeInvoke('trash_file', { path, projectId: projectId || null });
}

export async function deleteFilePermanent(path: string, projectId?: string): Promise<void> {
  return safeInvoke('delete_file_permanent', { path, projectId: projectId || null });
}

// ============= Favorites API =============

export async function addFavorite(projectId: string, fileId: string): Promise<void> {
  return safeInvoke('add_favorite', { projectId, fileId });
}

export async function removeFavorite(projectId: string, fileId: string): Promise<void> {
  return safeInvoke('remove_favorite', { projectId, fileId });
}

export async function getFavorites(projectId: string): Promise<string[]> {
  return safeInvoke('get_favorites', { projectId });
}

export async function isFavorite(projectId: string, fileId: string): Promise<boolean> {
  return safeInvoke('is_favorite', { projectId, fileId });
}

// ============= Tags API =============

export interface Tag {
  id: string;
  name: string;
  color: string;
}

export async function createTag(name: string, color: string): Promise<Tag> {
  return safeInvoke('create_tag', { name, color });
}

export async function listTags(): Promise<Tag[]> {
  return safeInvoke('list_tags');
}

export async function deleteTag(id: string): Promise<void> {
  return safeInvoke('delete_tag', { id });
}

export async function addFileTag(fileId: string, tagId: string): Promise<void> {
  return safeInvoke('add_file_tag', { fileId, tagId });
}

export async function removeFileTag(fileId: string, tagId: string): Promise<void> {
  return safeInvoke('remove_file_tag', { fileId, tagId });
}

export async function getFileTags(fileId: string): Promise<Tag[]> {
  return safeInvoke('get_file_tags', { fileId });
}

// ============= Advanced Search API =============

export interface SearchFilters {
  query?: string;
  fileTypes?: string[];
  minSize?: number;
  maxSize?: number;
  modifiedAfter?: string;
  modifiedBefore?: string;
  isDirectory?: boolean;
  tags?: string[];
}

export async function advancedSearch(projectId: string, filters: SearchFilters): Promise<FileNode[]> {
  return safeInvoke('advanced_search', { projectId, filters });
}

// ============= Batch Operations API =============

export interface BatchOperation {
  operation: 'move' | 'copy' | 'trash' | 'delete';
  paths: string[];
  destination?: string;
}

export interface BatchResult {
  success: boolean;
  processed: number;
  failed: number;
  errors: string[];
}

export async function batchOperation(operation: BatchOperation, projectId?: string): Promise<BatchResult> {
  return safeInvoke('batch_operation', { operation, projectId: projectId || null });
}

// ============= Backup/Sync API =============

export interface ProjectExport {
  project: Project;
  nodes: FileNode[];
  relations: FileRelation[];
  favorites: string[];
  tags: Tag[];
  fileTags: [string, string][];
}

export interface ProjectStats {
  totalFiles: number;
  totalDirs: number;
  totalSize: number;
  favoriteCount: number;
  nodeCount: number;
}

export async function exportProject(projectId: string): Promise<ProjectExport> {
  return safeInvoke('export_project', { projectId });
}

export async function importProject(data: ProjectExport): Promise<Project> {
  return safeInvoke('import_project', { data });
}

export async function getProjectStats(projectId: string): Promise<ProjectStats> {
  return safeInvoke('get_project_stats', { projectId });
}

export async function startFileWatcher(projectId: string, path: string): Promise<void> {
  return safeInvoke('start_file_watcher', { projectId, path });
}

export async function stopFileWatcher(projectId: string): Promise<void> {
  return safeInvoke('stop_file_watcher', { projectId });
}

interface ImportRelationResult {
  sourceId: string;
  sourceName: string;
  targetId: string;
  targetName: string;
  confidence: number;
}
