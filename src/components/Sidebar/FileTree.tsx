import { useState, useMemo, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { FileNode } from '../../types';
import { useAppStore } from '../../stores/appStore';

interface FlatItem {
  node: FileNode;
  depth: number;
}

/** Flatten the tree into a visible-item list respecting expansion state. */
function flattenVisibleNodes(
  nodes: FileNode[],
  expandedIds: Set<string>,
): FlatItem[] {
  const nodeMap = new Map<string, FileNode>();
  nodes.forEach((n) => nodeMap.set(n.id, n));

  const childrenMap = new Map<string, FileNode[]>();
  nodes.forEach((n) => {
    const pid = n.parentId || '';
    if (!childrenMap.has(pid)) childrenMap.set(pid, []);
    childrenMap.get(pid)!.push(n);
  });

  // Sort: directories first, then name
  const sortNodes = (list: FileNode[]) =>
    [...list].sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
      return a.name.localeCompare(b.name);
    });

  const result: FlatItem[] = [];

  function walk(pid: string, depth: number) {
    const kids = sortNodes(childrenMap.get(pid) || []);
    for (const n of kids) {
      result.push({ node: n, depth });
      if (n.isDirectory && expandedIds.has(n.id)) {
        walk(n.id, depth + 1);
      }
    }
  }

  walk('', 0);
  return result;
}

const getFileIcon = (ext: string, isDir: boolean): string => {
  if (isDir) return '📁';
  const map: Record<string, string> = {
    js: '📜', jsx: '⚛️', ts: '📘', tsx: '⚛️',
    py: '🐍', rs: '🦀', go: '🐰',
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

  const flatItems = useMemo(
    () => flattenVisibleNodes(nodes, expandedIds),
    [nodes, expandedIds],
  );

  const filteredItems = useMemo(() => {
    if (!filter) return flatItems;
    const q = filter.toLowerCase();
    return flatItems.filter((item) =>
      item.node.name.toLowerCase().includes(q),
    );
  }, [flatItems, filter]);

  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: filteredItems.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 24,
    overscan: 10,
  });

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleNodeClick = (node: FileNode) => {
    setSelectedNodeIds([node.id]);
    const vx = -node.positionX + 400;
    const vy = -node.positionY + 300;
    setViewport({ x: vx, y: vy, zoom: 1 });
    if (node.isDirectory) toggleExpand(node.id);
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
      <div ref={parentRef} className="max-h-[300px] overflow-y-auto px-1 pb-2">
        {filteredItems.length === 0 ? (
          <div className="text-center text-gray-500 text-xs py-4">暂无文件</div>
        ) : (
          <div
            style={{
              height: virtualizer.getTotalSize(),
              width: '100%',
              position: 'relative',
            }}
          >
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const item = filteredItems[virtualItem.index];
              const { node, depth } = item;
              const isExpanded = expandedIds.has(node.id);
              const hasChildren = nodes.some((n) => n.parentId === node.id);

              return (
                <div
                  key={virtualItem.key}
                  style={{
                    position: 'absolute',
                    top: virtualItem.start,
                    left: 0,
                    width: '100%',
                    height: virtualItem.size,
                  }}
                >
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
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
