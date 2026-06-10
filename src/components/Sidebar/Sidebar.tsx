import { useState, useEffect } from 'react';
import { useAppStore } from '../../stores/appStore';
import * as api from '../../services/api';
import { FileNode } from '../../types';

let openDialog: (() => Promise<string | null>) | null = null;

async function loadDialog() {
  if (!openDialog) {
    try {
      const dialog = await import('@tauri-apps/plugin-dialog');
      openDialog = async () => {
        const result = await dialog.open({
          directory: true,
          multiple: false,
          title: '选择项目文件夹',
        });
        return result as string | null;
      };
    } catch (e) {
      console.warn('Dialog plugin not available');
      return null;
    }
  }
  return openDialog;
}

export default function Sidebar() {
  const {
    projects,
    currentProject,
    setProjects,
    setCurrentProject,
    setFileNodes,
    setRelations,
    isLoading,
    setIsLoading,
  } = useAppStore();

  const [newProjectName, setNewProjectName] = useState('');
  const [showNewProject, setShowNewProject] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<FileNode[]>([]);
  const [isSearching, setIsSearching] = useState(false);

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

  const handleSelectProject = async (project: typeof projects[0]) => {
    setCurrentProject(project);
    setIsLoading(true);
    try {
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setFileNodes(nodes);
      setSearchResults([]);
      setSearchQuery('');
    } catch (error) {
      console.error('Failed to scan directory:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreateProject = async () => {
    if (!newProjectName.trim()) return;

    try {
      const dialogFn = await loadDialog();
      if (!dialogFn) {
        alert('请手动输入项目路径');
        return;
      }

      const selected = await dialogFn();

      if (selected) {
        setIsLoading(true);
        const project = await api.createProject(newProjectName, selected);
        setProjects([project, ...projects]);
        setCurrentProject(project);
        setNewProjectName('');
        setShowNewProject(false);

        const nodes = await api.scanDirectory(project.id, project.rootPath);
        setFileNodes(nodes);
        setIsLoading(false);
      }
    } catch (error) {
      console.error('Failed to create project:', error);
      setIsLoading(false);
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

    setIsLoading(true);
    try {
      const rels = await api.analyzeFileRelations(currentProject.id);
      setRelations(rels);
      alert(`分析完成，发现 ${rels.length} 个文件关联`);
    } catch (error) {
      console.error('Analysis failed:', error);
      alert('分析失败');
    } finally {
      setIsLoading(false);
    }
  };

  const handleClearSearch = () => {
    setSearchQuery('');
    setSearchResults([]);
  };

  const handleSearchResultClick = (node: FileNode) => {
    // TODO: Focus on the node in canvas
    console.log('Clicked:', node.name);
  };

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
          <button
            onClick={() => setShowNewProject(true)}
            className="text-lg hover:text-blue-400 transition-colors"
            title="新建项目"
          >
            +
          </button>
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
            <div className="flex gap-2">
              <button
                onClick={handleCreateProject}
                className="flex-1 px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-sm transition-colors"
              >
                创建
              </button>
              <button
                onClick={() => {
                  setShowNewProject(false);
                  setNewProjectName('');
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
