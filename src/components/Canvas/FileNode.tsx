import { memo, useState, useRef, useEffect } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { FileNode as FileNodeType } from '../../types';
import { useAppStore } from '../../stores/appStore';

// File type icons
const getFileIcon = (extension: string, isDirectory: boolean): string => {
  if (isDirectory) return '📁';
  
  const iconMap: Record<string, string> = {
    js: '📜', jsx: '⚛️', ts: '📘', tsx: '⚛️',
    py: '🐍', go: '🐹', java: '☕', rs: '🦀',
    c: '🔧', cpp: '🔧', h: '🔧', hpp: '🔧',
    html: '🌐', css: '🎨', scss: '🎨', less: '🎨',
    json: '📋', xml: '📋', yaml: '📋', yml: '📋',
    md: '📝', txt: '📝', doc: '📄', docx: '📄',
    png: '🖼️', jpg: '🖼️', jpeg: '🖼️', gif: '🖼️', svg: '🖼️', webp: '🖼️',
    mp4: '🎬', webm: '🎬', mkv: '🎬', mov: '🎬',
    mp3: '🎵', wav: '🎵', ogg: '🎵',
    zip: '📦', rar: '📦', '7z': '📦', tar: '📦', gz: '📦',
  };
  
  return iconMap[extension.toLowerCase()] || '📄';
};

const formatSize = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

function FileNodeComponent({ data, selected }: NodeProps) {
  const nodeData = data as unknown as FileNodeType;
  const { openTab, removeFileNode, updateFileNode } = useAppStore();
  
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭右键菜单
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    if (contextMenu) {
      document.addEventListener("mousedown", handleClickOutside);
      return () => document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [contextMenu]);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const handleOpen = () => {
    handleDoubleClick();
    setContextMenu(null);
  };

  const handleCopyPath = async () => {
    await navigator.clipboard.writeText(nodeData.path);
    setContextMenu(null);
  };

  const handleDelete = () => {
    if (confirm(`确定要删除 "${nodeData.name}" 吗？`)) {
      removeFileNode(nodeData.id);
    }
    setContextMenu(null);
  };


  const handleToggleCollapse = (e: React.MouseEvent) => {
    e.stopPropagation();
    updateFileNode(nodeData.id, { isCollapsed: !nodeData.isCollapsed });
  };

  const handleDoubleClick = () => {
    openTab({
      id: `tab-${nodeData.id}`,
      fileId: nodeData.id,
      path: nodeData.path,
      name: nodeData.name,
      type: nodeData.isDirectory ? 'viewer' : 'editor',
      isModified: false,
    });
  };

  return (
    <div
      className={`
        min-w-[180px] max-w-[220px] rounded-lg border-2 shadow-sm transition-all
        ${selected ? 'border-blue-500 shadow-lg' : 'border-gray-200 hover:border-gray-300'}
        ${nodeData.isDirectory ? 'bg-yellow-50' : 'bg-white'}
      `}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
    >
      <Handle type="target" position={Position.Top} className="!bg-gray-400" />
      
      <div className="p-3">
        <div className="flex items-center gap-2 mb-2">
          {nodeData.isDirectory && (
            <button
              onClick={handleToggleCollapse}
              className="w-4 h-4 flex items-center justify-center text-[10px] text-gray-500 hover:text-gray-800 transition-colors"
              title={nodeData.isCollapsed ? '展开' : '折叠'}
            >
              {nodeData.isCollapsed ? '▶' : '▼'}
            </button>
          )}
          <span className="text-xl">{getFileIcon(nodeData.extension, nodeData.isDirectory)}</span>
          <span className="font-medium text-gray-800 truncate flex-1" title={nodeData.name}>
            {nodeData.name}
          </span>
        </div>
        
        <div className="text-xs text-gray-500 space-y-1">
          {nodeData.extension && (
            <div className="flex items-center gap-1">
              <span className="px-1.5 py-0.5 bg-gray-100 rounded text-gray-600">
                .{nodeData.extension}
              </span>
            </div>
          )}
          
          <div className="flex items-center justify-between">
            <span>{formatSize(nodeData.size)}</span>
            {nodeData.relatedFiles && nodeData.relatedFiles.length > 0 && (
              <span className="text-blue-500">🔗 {nodeData.relatedFiles.length}</span>
            )}
          </div>
          
          {nodeData.tags && nodeData.tags.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-2">
              {nodeData.tags.slice(0, 3).map((tag, idx) => (
                <span
                  key={idx}
                  className="px-1.5 py-0.5 bg-blue-100 text-blue-700 rounded text-xs"
                >
                  {tag}
                </span>
              ))}
              {nodeData.tags.length > 3 && (
                <span className="text-gray-400">+{nodeData.tags.length - 3}</span>
              )}
            </div>
          )}
        </div>
      </div>
      
      <Handle type="source" position={Position.Bottom} className="!bg-gray-400" />
      {/* 右键菜单 */}
      {contextMenu && (
        <div
          ref={menuRef}
          className="fixed z-50 bg-white rounded-lg shadow-xl border border-gray-200 py-1 min-w-[160px]"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            className="w-full px-4 py-2 text-left text-sm hover:bg-gray-100 flex items-center gap-2"
            onClick={handleOpen}
          >
            <span>📂</span> 打开
          </button>
          <button
            className="w-full px-4 py-2 text-left text-sm hover:bg-gray-100 flex items-center gap-2"
            onClick={handleCopyPath}
          >
            <span>📋</span> 复制路径
          </button>
          <hr className="my-1 border-gray-200" />
          <button
            className="w-full px-4 py-2 text-left text-sm hover:bg-red-50 text-red-600 flex items-center gap-2"
            onClick={handleDelete}
          >
            <span>🗑️</span> 删除
          </button>
        </div>
      )}
    </div>
  );
}

export default memo(FileNodeComponent);
