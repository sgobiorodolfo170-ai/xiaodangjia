import { memo } from 'react';
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
  const { openTab } = useAppStore();
  
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
    >
      <Handle type="target" position={Position.Top} className="!bg-gray-400" />
      
      <div className="p-3">
        <div className="flex items-center gap-2 mb-2">
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
    </div>
  );
}

export default memo(FileNodeComponent);
