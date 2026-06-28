import { useAppStore } from '../../stores/appStore';
import * as api from '../../services/api';

export const ProjectList: React.FC = () => {
  const { projects, currentProject, setProjects, setCurrentProject, setFileNodes, startLoading, stopLoading, clearProjectState } = useAppStore();

  const handleSelectProject = async (project: typeof projects[0]) => {
    clearProjectState();
    setFileNodes([]);
    startLoading();
    try {
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setCurrentProject(project);
      setFileNodes(nodes);
    } catch (error) {
      console.error('Failed to scan directory:', error);
    } finally {
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

  if (projects.length === 0) {
    return (
      <div className="px-2 py-8 text-center text-gray-500 text-sm">
        <p>暂无项目</p>
      </div>
    );
  }

  return (
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
  );
};
