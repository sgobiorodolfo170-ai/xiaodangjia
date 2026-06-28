import { useState, useEffect, useCallback, useRef } from 'react';
import { useAppStore } from '../../stores/appStore';
import * as api from '../../services/api';
import { listen } from '@tauri-apps/api/event';
import { FileNode } from '../../types';
import FileTree from './FileTree';
import { FavoritesTagsPanel } from './FavoritesTagsPanel';
import { SearchFilterPanel } from './SearchFilterPanel';
import { StatsPanel } from './StatsPanel';
import { BackupPanel } from './BackupPanel';
import toast from 'react-hot-toast';

export default function Sidebar() {
  const {
    projects,
    currentProject,
    setProjects,
    setCurrentProject,
    setFileNodes,
    setRelations,
    isLoading,
    startLoading,
    stopLoading,
    fileNodes,
    setSelectedNodeIds,
    setViewport,
    clearProjectState,
  } = useAppStore();

  const [newProjectName, setNewProjectName] = useState('');
  const [showNewProject, setShowNewProject] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<FileNode[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [showFavoritesPanel, setShowFavoritesPanel] = useState(false);
  const [showAdvancedSearch, setShowAdvancedSearch] = useState(false);
  const [showStatsPanel, setShowStatsPanel] = useState(false);
  const [showBackupPanel, setShowBackupPanel] = useState(false);
  
  // 新建项目相关状态
  const [folderSource, setFolderSource] = useState<'existing' | 'new'>('existing');
  const [parentFolderPath, setParentFolderPath] = useState('');
  const [newFolderName, setNewFolderName] = useState('');
  
  const rescanTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    loadProjects();
  }, []);

  const loadProjects = async () => {
    try {
      const list = await api.listProjects();
      setProjects(list);
    } catch (error) {
      console.error('Failed to load projects:', error);
    }
  };

  // Debounced re-scan on file changes — incremental merge to preserve user layout
  const debouncedRescan = useCallback(async () => {
    if (rescanTimerRef.current) {
      clearTimeout(rescanTimerRef.current);
    }
    rescanTimerRef.current = setTimeout(async () => {
      if (!currentProject) return;
      try {
        const freshNodes = await api.scanDirectory(currentProject.id, currentProject.rootPath);
        // Build a path→existing-node map to preserve user layout
        const existingMap = new Map<string, FileNode>();
        for (const node of fileNodes) {
          existingMap.set(node.path, node);
        }
        // Merge: keep existing positionX/Y/isCollapsed/tags, update size/modifiedAt,
        // add new nodes (at scan-assigned positions), remove deleted nodes
        const merged: FileNode[] = freshNodes.map((fresh) => {
          const existing = existingMap.get(fresh.path);
          if (existing) {
            return {
              ...fresh,
              positionX: existing.positionX,
              positionY: existing.positionY,
              isCollapsed: existing.isCollapsed,
            };
          }
          return fresh; // new node — keep scan-assigned position
        });
        setFileNodes(merged);
      } catch (e) {
        console.warn('Rescan failed:', e);
      }
    }, 2000); // 2s debounce
  }, [currentProject, fileNodes, setFileNodes]);

  // 监听文件变更，自动刷新画布
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupWatcher = async () => {
      if (!currentProject) return;

      try {
        // 启动文件监听
        await api.startFileWatcher(currentProject.id, currentProject.rootPath);

        // 监听文件变更事件
        unlisten = await listen<{ event_type: string; path: string; is_directory: boolean }>('file-change', async () => {
          debouncedRescan();
        });
      } catch (e) {
        console.warn('File watcher not available:', e);
      }
    };

    setupWatcher();

    return () => {
      if (unlisten) unlisten();
      if (rescanTimerRef.current) {
        clearTimeout(rescanTimerRef.current);
      }
      // Stop the watcher when switching away from this project
      if (currentProject) {
        api.stopFileWatcher(currentProject.id).catch((e) => {
          console.warn('Failed to stop watcher:', e);
        });
      }
    };
  }, [currentProject?.id]);

  const handleSelectProject = async (project: typeof projects[0]) => {
    clearProjectState();
    setFileNodes([]);
    startLoading();
    try {
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setCurrentProject(project);
      setFileNodes(nodes);
      setSearchResults([]);
      setSearchQuery('');
    } catch (error) {
      console.error('Failed to scan directory:', error);
    } finally {
      stopLoading();
    }
  };

  // 选择父目录
  const handleSelectParentFolder = async () => {
    try {
      const selected = await api.openDirectoryDialog();
      if (selected) {
        setParentFolderPath(selected);
      }
    } catch (error) {
      console.error('Failed to select parent folder:', error);
    }
  };

  // 判断是否可以创建项目
  const canCreateProject = () => {
    if (!newProjectName.trim()) return false;
    if (folderSource === 'existing') return true;
    if (folderSource === 'new') return parentFolderPath && newFolderName.trim();
    return false;
  };

  // 快速创建项目：直接弹出文件夹选择对话框，使用文件夹名作为项目名
  const handleQuickCreateProject = async () => {
    try {
      const selectedPath = await api.openDirectoryDialog();
      if (!selectedPath) return;

      // 从路径中提取文件夹名作为项目名
      const pathParts = selectedPath.replace(/\\/g, '/').split('/');
      const folderName = pathParts[pathParts.length - 1] || '新项目';

      startLoading();

      // 创建项目
      const project = await api.createProject(folderName, selectedPath);
      setProjects([project, ...projects]);
      setCurrentProject(project);

      // 扫描目录
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setFileNodes(nodes);
      stopLoading();
    } catch (error) {
      console.error('Failed to create project:', error);
      stopLoading();
    }
  };

  const handleCreateProject = async () => {
    if (!newProjectName.trim()) return;

    try {
      let rootPath: string;

      if (folderSource === 'new') {
        // 创建新文件夹模式：直接使用已选择的路径
        // Windows 路径拼接
        rootPath = parentFolderPath.endsWith('\\') 
          ? parentFolderPath + newFolderName 
          : parentFolderPath + '\\' + newFolderName;
        
        startLoading();
        // 先创建文件夹
        await api.createDirectory(rootPath);
      } else {
        // 选择已有文件夹模式：弹出对话框让用户选择
        const selected = await api.openDirectoryDialog();
        if (!selected) return;
        rootPath = selected;
        startLoading();
      }

      // 创建项目
      const project = await api.createProject(newProjectName, rootPath);
      setProjects([project, ...projects]);
      setCurrentProject(project);
      
      // 重置表单状态
      setNewProjectName('');
      setShowNewProject(false);
      setFolderSource('existing');
      setParentFolderPath('');
      setNewFolderName('');

      // 扫描目录
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setFileNodes(nodes);
      stopLoading();
    } catch (error) {
      console.error('Failed to create project:', error);
      stopLoading();
    }
  };

  const handleDeleteProject = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm('确定要删除这个项目吗？')) return;

    try {
      await api.deleteProject(id);
      setProjects(projects.filter((p) => p.id !== id));
      if (currentProject?.id === id) {
        setCurrentProject(null);
        setFileNodes([]);
      }
    } catch (error) {
      console.error('Failed to delete project:', error);
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim() || !currentProject) return;

    setIsSearching(true);
    try {
      const results = await api.searchFiles(currentProject.id, searchQuery);
      setSearchResults(results);
    } catch (error) {
      console.error('Search failed:', error);
    } finally {
      setIsSearching(false);
    }
  };

  const handleAnalyzeRelations = async () => {
    if (!currentProject) return;

    startLoading();
    try {
      const rels = await api.analyzeFileRelations(currentProject.id);
      setRelations(rels);
      toast.success(`分析完成，发现 ${rels.length} 个文件关联`);
    } catch (error) {
      console.error('Analysis failed:', error);
      toast.error('分析失败');
    } finally {
      stopLoading();
    }
  };

  const handleClearSearch = () => {
    setSearchQuery('');
    setSearchResults([]);
  };

  const handleSearchResultClick = useCallback((node: FileNode) => {
    // 选中节点并定位到画布
    setSelectedNodeIds([node.id]);

    // 将视口居中到该节点
    const viewportX = -node.positionX + 400;
    const viewportY = -node.positionY + 300;
    setViewport({ x: viewportX, y: viewportY, zoom: 1 });
  }, [setSelectedNodeIds, setViewport]);

  return (
    <div className="w-64 h-full bg-gray-900 text-white flex flex-col">
      <div className="p-4 border-b border-gray-700">
        <h1 className="text-xl font-bold flex items-center gap-2">
          <span>🧠</span>
          <span>小当家</span>
        </h1>
        <p className="text-xs text-gray-400 mt-1">脑图式文件管理器</p>
      </div>

      {/* Search Bar */}
      {currentProject && (
        <div className="p-2 border-b border-gray-700">
          <div className="flex gap-1">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
              placeholder="搜索文件..."
              className="flex-1 px-2 py-1 bg-gray-800 border border-gray-700 rounded text-sm focus:outline-none focus:border-blue-500"
            />
            <button
              onClick={handleSearch}
              disabled={isSearching}
              className="px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-sm"
            >
              {isSearching ? '...' : '🔍'}
            </button>
            {searchResults.length > 0 && (
              <button
                onClick={handleClearSearch}
                className="px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm"
              >
                ✕
              </button>
            )}
          </div>
        </div>
      )}

      {/* AI Analysis Button */}
      {currentProject && (
        <div className="p-2 border-b border-gray-700">
          <button
            onClick={handleAnalyzeRelations}
            disabled={isLoading}
            className="w-full px-2 py-2 bg-gradient-to-r from-purple-600 to-blue-600 hover:from-purple-700 hover:to-blue-700 rounded text-sm flex items-center justify-center gap-2 transition-all"
          >
            <span>🤖</span>
            <span>AI 分析关联</span>
          </button>
        </div>
      )}

      {/* Favorites/Tags Toggle */}
      <div className="p-2 border-b border-gray-700 flex gap-2 flex-wrap">
        {currentProject && (
          <>
            <button
              onClick={() => {
                setShowFavoritesPanel(!showFavoritesPanel);
                setShowAdvancedSearch(false);
                setShowStatsPanel(false);
                setShowBackupPanel(false);
              }}
              className={`flex-1 px-2 py-2 rounded text-sm flex items-center justify-center gap-2 transition-all min-w-[80px] ${
                showFavoritesPanel 
                  ? 'bg-yellow-600 text-white' 
                  : 'bg-gray-700 hover:bg-gray-600'
              }`}
            >
              <span>⭐</span>
              <span>收藏</span>
            </button>
            <button
              onClick={() => {
                setShowAdvancedSearch(!showAdvancedSearch);
                setShowFavoritesPanel(false);
                setShowStatsPanel(false);
                setShowBackupPanel(false);
              }}
              className={`flex-1 px-2 py-2 rounded text-sm flex items-center justify-center gap-2 transition-all min-w-[80px] ${
                showAdvancedSearch 
                  ? 'bg-green-600 text-white' 
                  : 'bg-gray-700 hover:bg-gray-600'
              }`}
            >
              <span>🔍</span>
              <span>搜索</span>
            </button>
            <button
              onClick={() => {
                setShowStatsPanel(!showStatsPanel);
                setShowFavoritesPanel(false);
                setShowAdvancedSearch(false);
                setShowBackupPanel(false);
              }}
              className={`flex-1 px-2 py-2 rounded text-sm flex items-center justify-center gap-2 transition-all min-w-[80px] ${
                showStatsPanel 
                  ? 'bg-purple-600 text-white' 
                  : 'bg-gray-700 hover:bg-gray-600'
              }`}
            >
              <span>📊</span>
              <span>统计</span>
            </button>
          </>
        )}
        <button
          onClick={() => {
            setShowBackupPanel(!showBackupPanel);
            setShowFavoritesPanel(false);
            setShowAdvancedSearch(false);
            setShowStatsPanel(false);
          }}
          className={`flex-1 px-2 py-2 rounded text-sm flex items-center justify-center gap-2 transition-all min-w-[80px] ${
            showBackupPanel 
              ? 'bg-teal-600 text-white' 
              : 'bg-gray-700 hover:bg-gray-600'
          }`}
        >
          <span>💾</span>
          <span>备份</span>
        </button>
      </div>

      {/* Advanced Search Panel */}
      {showAdvancedSearch && (
        <SearchFilterPanel onResults={(results) => {
          setSearchResults(results);
          setSearchQuery('');
        }} />
      )}

      {/* Stats Panel */}
      {showStatsPanel && (
        <div className="flex-1 overflow-hidden">
          <StatsPanel />
        </div>
      )}

      {/* Backup Panel */}
      {showBackupPanel && (
        <div className="flex-1 overflow-hidden">
          <BackupPanel />
        </div>
      )}

      {/* File Tree */}
      {currentProject && fileNodes.length > 0 && !showFavoritesPanel && (
        <FileTree nodes={fileNodes} />
      )}

      {/* Favorites/Tags Panel */}
      {currentProject && showFavoritesPanel && (
        <div className="flex-1 overflow-hidden">
          <FavoritesTagsPanel 
            onFileSelect={(fileId) => {
              handleSearchResultClick(fileNodes.find(n => n.id === fileId)!);
            }}
          />
        </div>
      )}

      {/* Search Results */}
      {searchResults.length > 0 && (
        <div className="p-2 border-b border-gray-700 max-h-48 overflow-y-auto">
          <div className="text-xs text-gray-400 mb-1">搜索结果 ({searchResults.length})</div>
          <div className="space-y-1">
            {searchResults.map((node: FileNode) => (
              <div
                key={node.id}
                onClick={() => handleSearchResultClick(node)}
                className="px-2 py-1 bg-gray-800 rounded text-sm truncate cursor-pointer hover:bg-gray-700"
                title={node.path}
              >
                📄 {node.name}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Projects List */}
      <div className="flex-1 overflow-y-auto p-2">
        <div className="flex items-center justify-between px-2 py-2">
          <span className="text-sm text-gray-400">项目列表</span>
          <div className="flex gap-2">
            <button
              onClick={handleQuickCreateProject}
              className="text-sm bg-blue-600 hover:bg-blue-700 px-2 py-1 rounded transition-colors"
              title="选择文件夹创建项目"
            >
              + 添加项目
            </button>
            <button
              onClick={() => setShowNewProject(true)}
              className="text-lg hover:text-blue-400 transition-colors"
              title="新建项目"
            >
              ⚙
            </button>
          </div>
        </div>

        {showNewProject && (
          <div className="px-2 py-2 mb-2 bg-gray-800 rounded-lg">
            <input
              type="text"
              value={newProjectName}
              onChange={(e) => setNewProjectName(e.target.value)}
              placeholder="项目名称"
              className="w-full px-2 py-1 mb-2 bg-gray-700 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
              onKeyDown={(e) => e.key === 'Enter' && handleCreateProject()}
              autoFocus
            />
            
            {/* 文件夹来源选择 */}
            <div className="mb-2 text-sm text-gray-300">
              <div className="mb-1">文件夹来源:</div>
              <label className="flex items-center gap-2 cursor-pointer hover:text-white">
                <input
                  type="radio"
                  name="folderSource"
                  checked={folderSource === 'existing'}
                  onChange={() => {
                    setFolderSource('existing');
                    setParentFolderPath('');
                    setNewFolderName('');
                  }}
                  className="accent-blue-500"
                />
                选择已有文件夹
              </label>
              <label className="flex items-center gap-2 cursor-pointer hover:text-white">
                <input
                  type="radio"
                  name="folderSource"
                  checked={folderSource === 'new'}
                  onChange={() => setFolderSource('new')}
                  className="accent-blue-500"
                />
                创建新文件夹
              </label>
            </div>
            
            {/* 选择已有文件夹时 */}
            {folderSource === 'existing' && (
              <div className="mb-2 text-xs text-gray-400">
                点击"创建"后将弹出文件夹选择对话框
              </div>
            )}
            
            {/* 创建新文件夹时 */}
            {folderSource === 'new' && (
              <div className="space-y-2 mb-2">
                <button
                  type="button"
                  onClick={handleSelectParentFolder}
                  className="w-full px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm text-left"
                >
                  {parentFolderPath ? `父目录: ${parentFolderPath}` : '点击选择父目录'}
                </button>
                {parentFolderPath && (
                  <input
                    type="text"
                    value={newFolderName}
                    onChange={(e) => setNewFolderName(e.target.value)}
                    placeholder="新文件夹名称"
                    className="w-full px-2 py-1 bg-gray-700 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
                  />
                )}
              </div>
            )}
            
            <div className="flex gap-2">
              <button
                onClick={handleCreateProject}
                disabled={!canCreateProject()}
                className={`flex-1 px-2 py-1 rounded text-sm transition-colors ${
                  canCreateProject() 
                    ? 'bg-blue-600 hover:bg-blue-700' 
                    : 'bg-gray-600 cursor-not-allowed'
                }`}
              >
                创建
              </button>
              <button
                onClick={() => {
                  setShowNewProject(false);
                  setNewProjectName('');
                  setFolderSource('existing');
                  setParentFolderPath('');
                  setNewFolderName('');
                }}
                className="flex-1 px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm transition-colors"
              >
                取消
              </button>
            </div>
          </div>
        )}

        {projects.length === 0 ? (
          <div className="px-2 py-8 text-center text-gray-500 text-sm">
            <p>暂无项目</p>
            <p className="mt-2">点击 + 创建新项目</p>
          </div>
        ) : (
          <div className="space-y-1">
            {projects.map((project) => (
              <div
                key={project.id}
                onClick={() => handleSelectProject(project)}
                className={`
                  group flex items-center justify-between px-3 py-2 rounded-lg cursor-pointer transition-colors
                  ${currentProject?.id === project.id
                    ? 'bg-blue-600 text-white'
                    : 'hover:bg-gray-800 text-gray-300'
                  }
                `}
              >
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  <span>📁</span>
                  <span className="truncate">{project.name}</span>
                </div>
                <button
                  onClick={(e) => handleDeleteProject(project.id, e)}
                  className="opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-400 transition-all"
                  title="删除项目"
                >
                  🗑️
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {isLoading && (
        <div className="p-3 border-t border-gray-700">
          <div className="flex items-center gap-2 text-sm text-blue-400">
            <span className="animate-spin">⏳</span>
            <span>处理中...</span>
          </div>
        </div>
      )}

      <div className="p-3 border-t border-gray-700 text-xs text-gray-500">
        <p>共 {projects.length} 个项目</p>
        {currentProject && <p className="mt-1">当前: {currentProject.name}</p>}
      </div>
    </div>
  );
}
