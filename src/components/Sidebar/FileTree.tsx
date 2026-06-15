import { useState, useMemo } from 'react';
import { FileNode } from '../../types';
import { useAppStore } from '../../stores/appStore';

interface TreeNode {
  node: FileNode;
  children: TreeNode[];
  depth: number;
}

function buildTree(nodes: FileNode[]): TreeNode[] {
  const nodeMap = new Map<string, FileNode>();
  nodes.forEach((n) => nodeMap.set(n.id, n));

  // Build parent-child relationships
  const childrenMap = new Map<string, FileNode[]>();
  nodes.forEach((n) => {
    const parentId = n.parentId || '';
    if (!childrenMap.has(parentId)) {
      childrenMap.set(parentId, []);
    }
    childrenMap.get(parentId)!.push(n);
  });

  // Sort: directories first, then by name
  const sortNodes = (list: FileNode[]) => {
    return [...list].sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) {
        return a.isDirectory ? -1 : 1;
      }
      return a.name.localeCompare(b.name);
    });
  };

  function build(pid: string, depth: number): TreeNode[] {
    const kids = childrenMap.get(pid) || [];
    return sortNodes(kids).map((n) => ({
      node: n,
      children: build(n.id, depth + 1),
      depth,
    }));
  }

  return build('', 0);
}

const getFileIcon = (ext: string, isDir: boolean): string => {
  if (isDir) return '📁';
  const map: Record<string, string> = {
    js: '📜', jsx: '⚛️', ts: '📘', tsx: '⚛️',
    py: '🐍', rs: '🦀', go: '🐹',
    html: '🌐', css: '🎨', json: '📋',
    md: '📝', txt: '📝',
    png: '🖼️', jpg: '🖼️', svg: '🖼️',
  };
  return map[ext.toLowerCase()] || '📄';
};

interface FileTreeProps {
  nodes: FileNode[];
}

export default function FileTree({ nodes }: FileTreeProps) {
  const { setSelectedNodeIds, setViewport } = useAppStore();
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState('');

  const tree = useMemo(() => buildTree(nodes), [nodes]);

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleNodeClick = (node: FileNode) => {
    setSelectedNodeIds([node.id]);
    const vx = -node.positionX + 400;
    const vy = -node.positionY + 300;
    setViewport({ x: vx, y: vy, zoom: 1 });

    if (node.isDirectory) {
      toggleExpand(node.id);
    }
  };

  const renderNode = (item: TreeNode) => {
    const { node, children, depth } = item;
    const isExpanded = expandedIds.has(node.id);
    const hasChildren = children.length > 0;

    // Filter
    if (filter && !node.name.toLowerCase().includes(filter.toLowerCase())) {
      // If filter is active and this node doesn't match, check children
      const hasMatchingChild = children.some((c) =>
        c.node.name.toLowerCase().includes(filter.toLowerCase())
      );
      if (!hasMatchingChild) return null;
    }

    return (
      <div key={node.id}>
        <div
          className="flex items-center gap-1 px-1 py-0.5 rounded cursor-pointer hover:bg-gray-700 text-xs truncate"
          style={{ paddingLeft: `${depth * 12 + 4}px` }}
          onClick={() => handleNodeClick(node)}
          title={node.path}
        >
          {hasChildren ? (
            <span className="w-3 text-center text-gray-400 text-[10px]">
              {isExpanded ? '▼' : '▶'}
            </span>
          ) : (
            <span className="w-3" />
          )}
          <span className="flex-shrink-0">{getFileIcon(node.extension, node.isDirectory)}</span>
          <span className="truncate flex-1">{node.name}</span>
        </div>
        {hasChildren && isExpanded && (
          <div>
            {children.map((child) => renderNode(child))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="border-t border-gray-700">
      <div className="flex items-center justify-between px-2 py-1.5">
        <span className="text-xs text-gray-400">文件树</span>
        <input
          type="text"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="过滤..."
          className="w-20 px-1 py-0.5 bg-gray-800 border border-gray-700 rounded text-[10px] focus:outline-none focus:border-blue-500 text-gray-300"
        />
      </div>
      <div className="max-h-[300px] overflow-y-auto px-1 pb-2">
        {tree.length === 0 ? (
          <div className="text-center text-gray-500 text-xs py-4">暂无文件</div>
        ) : (
          tree.map((item) => renderNode(item))
        )}
      </div>
    </div>
  );
}
