import React, { useState } from 'react';
import { useAppStore } from '../../stores/appStore';
import { exportProject, importProject, ProjectExport } from '../../services/api';
import * as api from '../../services/api';

export const BackupPanel: React.FC = () => {
  const { currentProject, setCurrentProject, setFileNodes, setProjects } = useAppStore();
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const handleExport = async () => {
    if (!currentProject) return;
    
    setIsExporting(true);
    setMessage(null);
    try {
      const data = await exportProject(currentProject.id);
      
      // Create JSON file and trigger download
      const jsonStr = JSON.stringify(data, null, 2);
      const blob = new Blob([jsonStr], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      
      const a = document.createElement('a');
      a.href = url;
      a.download = `${currentProject.name.replace(/[^a-z0-9]/gi, '_')}_backup_${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      
      setMessage({ type: 'success', text: '项目导出成功！' });
    } catch (error) {
      console.error('Export failed:', error);
      setMessage({ type: 'error', text: '导出失败: ' + String(error) });
    } finally {
      setIsExporting(false);
    }
  };

  const handleImport = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      
      setIsImporting(true);
      setMessage(null);
      try {
        const text = await file.text();
        const data = JSON.parse(text) as ProjectExport;
        
        // Validate basic structure
        if (!data.project || !data.nodes) {
          throw new Error('无效的备份文件格式');
        }
        
        const newProject = await importProject(data);
        
        // Refresh project list
        const projectList = await api.listProjects();
        setProjects(projectList);
        
        // Select the imported project
        setCurrentProject(newProject);
        const nodes = await api.scanDirectory(newProject.id, newProject.rootPath);
        setFileNodes(nodes);
        
        setMessage({ type: 'success', text: `项目 "${newProject.name}" 导入成功！` });
      } catch (error) {
        console.error('Import failed:', error);
        setMessage({ type: 'error', text: '导入失败: ' + String(error) });
      } finally {
        setIsImporting(false);
      }
    };
    input.click();
  };

  const handleNewProject = async () => {
    try {
      const selected = await api.openDirectoryDialog();
      if (!selected) return;
      
      const name = selected.split(/[/\\]/).pop() || '新项目';
      const project = await api.createProject(name, selected);
      
      const projectList = await api.listProjects();
      setProjects(projectList);
      setCurrentProject(project);
      
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      setFileNodes(nodes);
      
      setMessage({ type: 'success', text: '项目创建成功！' });
    } catch (error) {
      console.error('Create project failed:', error);
      setMessage({ type: 'error', text: '创建项目失败: ' + String(error) });
    }
  };

  return (
    <div className="h-full overflow-y-auto bg-white dark:bg-gray-800 p-4">
      <h3 className="font-bold text-lg mb-4 flex items-center gap-2">
        <span>💾</span>
        <span>备份与同步</span>
      </h3>

      {/* Message */}
      {message && (
        <div className={`mb-4 p-3 rounded text-sm ${
          message.type === 'success' 
            ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
            : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
        }`}>
          {message.text}
        </div>
      )}

      {/* Current Project Actions */}
      {currentProject && (
        <div className="mb-6">
          <h4 className="font-semibold text-sm mb-3 flex items-center gap-1">
            <span>📁</span>
            当前项目
          </h4>
          <div className="bg-gray-50 dark:bg-gray-700 p-3 rounded-lg mb-3">
            <p className="font-medium">{currentProject.name}</p>
            <p className="text-xs text-gray-500 dark:text-gray-400 truncate" title={currentProject.rootPath}>
              {currentProject.rootPath}
            </p>
          </div>
          
          <button
            onClick={handleExport}
            disabled={isExporting}
            className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-400 text-white rounded-lg text-sm font-medium flex items-center justify-center gap-2 mb-2"
          >
            {isExporting ? '⏳ 导出中...' : '📤 导出项目'}
          </button>
          <p className="text-xs text-gray-500 mb-4">
            导出项目配置、节点、收藏和标签到 JSON 文件
          </p>
        </div>
      )}

      {/* Import */}
      <div className="mb-6">
        <h4 className="font-semibold text-sm mb-3 flex items-center gap-1">
          <span>📥</span>
          导入备份
        </h4>
        <button
          onClick={handleImport}
          disabled={isImporting}
          className="w-full px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-green-400 text-white rounded-lg text-sm font-medium flex items-center justify-center gap-2 mb-2"
        >
          {isImporting ? '⏳ 导入中...' : '📥 从备份文件导入'}
        </button>
        <p className="text-xs text-gray-500">
          从之前导出的 JSON 备份文件恢复项目
        </p>
      </div>

      {/* New Project */}
      <div className="border-t pt-4">
        <h4 className="font-semibold text-sm mb-3 flex items-center gap-1">
          <span>➕</span>
          新建项目
        </h4>
        <button
          onClick={handleNewProject}
          className="w-full px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-sm font-medium flex items-center justify-center gap-2"
        >
          <span>➕</span>
          创建新项目
        </button>
        <p className="text-xs text-gray-500 mt-2">
          从文件夹创建新项目
        </p>
      </div>

      {/* Help */}
      <div className="mt-6 p-3 bg-gray-50 dark:bg-gray-700 rounded-lg">
        <h5 className="font-medium text-sm mb-2">💡 使用提示</h5>
        <ul className="text-xs text-gray-600 dark:text-gray-400 space-y-1">
          <li>• 导出项目可将脑图配置保存为文件</li>
          <li>• 导入备份可在不同设备间同步</li>
          <li>• 备份包含节点位置、收藏和标签</li>
          <li>• 不包含实际文件内容</li>
        </ul>
      </div>
    </div>
  );
};