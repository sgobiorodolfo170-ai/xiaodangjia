import { useState } from 'react';
import { useAppStore } from '../../stores/appStore';
import * as api from '../../services/api';
interface ProjectCreatorProps {
  onCreated: () => void;
}

export const ProjectCreator: React.FC<ProjectCreatorProps> = ({ onCreated }) => {
  const { projects, setProjects, setCurrentProject, setFileNodes, startLoading, stopLoading } = useAppStore();

  const [showNewProject, setShowNewProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState('');
  const [folderSource, setFolderSource] = useState<'existing' | 'new'>('existing');
  const [parentFolderPath, setParentFolderPath] = useState('');
  const [newFolderName, setNewFolderName] = useState('');

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

  const canCreateProject = () => {
    if (!newProjectName.trim()) return false;
    if (folderSource === 'existing') return true;
    if (folderSource === 'new') return parentFolderPath && newFolderName.trim();
    return false;
  };

  const handleQuickCreateProject = async () => {
    try {
      const selectedPath = await api.openDirectoryDialog();
      if (!selectedPath) return;

      const pathParts = selectedPath.replace(/\\/g, '/').split('/');
      const folderName = pathParts[pathParts.length - 1] || '新项目';

      startLoading();
      const project = await api.createProject(folderName, selectedPath);
      setProjects([project, ...projects]);
      setCurrentProject(project);

      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setFileNodes(nodes);
      stopLoading();
      onCreated();
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
        rootPath = parentFolderPath.endsWith('\\')
          ? parentFolderPath + newFolderName
          : parentFolderPath + '\\' + newFolderName;

        startLoading();
        await api.createDirectory(rootPath);
      } else {
        const selected = await api.openDirectoryDialog();
        if (!selected) return;
        rootPath = selected;
        startLoading();
      }

      const project = await api.createProject(newProjectName, rootPath);
      setProjects([project, ...projects]);
      setCurrentProject(project);

      setNewProjectName('');
      setShowNewProject(false);
      setFolderSource('existing');
      setParentFolderPath('');
      setNewFolderName('');

      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setFileNodes(nodes);
      stopLoading();
      onCreated();
    } catch (error) {
      console.error('Failed to create project:', error);
      stopLoading();
    }
  };

  const cancelCreate = () => {
    setShowNewProject(false);
    setNewProjectName('');
    setFolderSource('existing');
    setParentFolderPath('');
    setNewFolderName('');
  };

  return (
    <div className="flex gap-2">
      <button
        onClick={handleQuickCreateProject}
        className="text-sm bg-blue-600 hover:bg-blue-700 px-2 py-1 rounded"
        title="选择文件夹创建项目"
      >
        + 添加项目
      </button>
      <button
        onClick={() => setShowNewProject(true)}
        className="text-lg hover:text-blue-400"
        title="新建项目"
      >
        ⚙
      </button>

      {showNewProject && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={cancelCreate}>
          <div className="bg-gray-800 rounded-lg p-4 w-96 shadow-xl" onClick={(e) => e.stopPropagation()}>
            <input
              type="text"
              value={newProjectName}
              onChange={(e) => setNewProjectName(e.target.value)}
              placeholder="项目名称"
              className="w-full px-2 py-1 mb-2 bg-gray-700 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
              onKeyDown={(e) => e.key === 'Enter' && handleCreateProject()}
              autoFocus
            />

            <div className="mb-2 text-sm text-gray-300">
              <div className="mb-1">文件夹来源:</div>
              <label className="flex items-center gap-2 cursor-pointer hover:text-white">
                <input
                  type="radio"
                  name="folderSource"
                  checked={folderSource === 'existing'}
                  onChange={() => { setFolderSource('existing'); setParentFolderPath(''); setNewFolderName(''); }}
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

            {folderSource === 'existing' && (
              <div className="mb-2 text-xs text-gray-400">
                点击"创建"后将弹出文件夹选择对话框
              </div>
            )}

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
                onClick={cancelCreate}
                className="flex-1 px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm"
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
