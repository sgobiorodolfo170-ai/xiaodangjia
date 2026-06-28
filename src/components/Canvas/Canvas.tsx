import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  useReactFlow,
  ReactFlowProvider,
  Node,
  Edge,
  applyNodeChanges,
  OnNodesChange,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useAppStore } from '../../stores/appStore';
import { FileNode } from '../../types';
import FileNodeComponent from './FileNode';
import { BatchToolbar } from './BatchToolbar';
import toast from 'react-hot-toast';

const nodeTypes = {
  file: FileNodeComponent,
};

// 内部组件 - 使用 useReactFlow hook
function CanvasInner() {
  const {
    fileNodes,
    relations,
    currentProject,
    viewport,
    setViewport,
    setSelectedNodeIds,
    updateNodePosition,
    darkMode,
    setFileNodes,
    isLoading,
  } = useAppStore();

  const [isDragging, setIsDragging] = useState(false);
  const { screenToFlowPosition, fitView } = useReactFlow();
  // Track whether nodes have been initialized for the current project so we
  // only fit the view on first load, not on every fileNodes change.
  const fitDoneForProjectRef = useRef<string | null>(null);

  // Build set of collapsed directory IDs
  const collapsedDirIds = useMemo(() => {
    return new Set(
      fileNodes.filter((n) => n.isDirectory && n.isCollapsed).map((n) => n.id)
    );
  }, [fileNodes]);

  // Find all descendants of collapsed directories (recursively)
  const hiddenNodeIds = useMemo(() => {
    const hidden = new Set<string>();
    const parentMap = new Map<string, string[]>();
    fileNodes.forEach((n) => {
      const pid = n.parentId || '';
      if (!parentMap.has(pid)) parentMap.set(pid, []);
      parentMap.get(pid)!.push(n.id);
    });

    const collectDescendants = (ids: string[]) => {
      for (const id of ids) {
        if (hidden.has(id)) continue;
        hidden.add(id);
        const children = parentMap.get(id) || [];
        collectDescendants(children);
      }
    };

    collectDescendants(Array.from(collapsedDirIds));
    return hidden;
  }, [fileNodes, collapsedDirIds]);

  // Convert fileNodes to React Flow nodes, filtering hidden ones (memoized)
  const rfNodes: Node[] = useMemo(
    () =>
      fileNodes
        .filter((node) => !hiddenNodeIds.has(node.id))
        .map((node) => ({
          id: node.id,
          type: 'file',
          position: { x: node.positionX, y: node.positionY },
          data: { ...node },
        })),
    [fileNodes, hiddenNodeIds]
  );

  // Convert relations to React Flow edges, filtering hidden ones (memoized)
  const rfEdges: Edge[] = useMemo(
    () =>
      relations
        .filter((rel) => !hiddenNodeIds.has(rel.sourceId) && !hiddenNodeIds.has(rel.targetId))
        .map((rel) => ({
          id: rel.id,
          source: rel.sourceId,
          target: rel.targetId,
          type: 'smoothstep',
          animated: rel.confidence > 0.7,
          label: rel.relationType,
          style: {
            stroke: rel.confidence > 0.7 ? '#22c55e' : '#94a3b8',
            strokeWidth: Math.max(1, rel.confidence * 3),
          },
        })),
    [relations, hiddenNodeIds]
  );

  const [nodes, setNodes] = useNodesState(rfNodes);
  const [edges, setEdges] = useEdgesState(rfEdges);

  // Update nodes when fileNodes change — smooth replacement, no fitView jump on drag.
  useEffect(() => {
    setNodes(rfNodes);
  }, [rfNodes, setNodes]);

  // Update edges when relations change
  useEffect(() => {
    setEdges(rfEdges);
  }, [rfEdges, setEdges]);

  // Fit the view once per project load (not on every render) with a smooth transition.
  useEffect(() => {
    if (!currentProject) {
      fitDoneForProjectRef.current = null;
      return;
    }
    if (
      fitDoneForProjectRef.current !== currentProject.id &&
      rfNodes.length > 0 &&
      !isLoading
    ) {
      fitDoneForProjectRef.current = currentProject.id;
      // Defer to next frame so ReactFlow has nodes laid out.
      const timer = setTimeout(() => {
        fitView({ duration: 400, padding: 0.2 });
      }, 60);
      return () => clearTimeout(timer);
    }
  }, [currentProject, rfNodes.length, isLoading, fitView]);

  const fileNodeIds = useMemo(() => new Set(fileNodes.map((n) => n.id)), [fileNodes]);

  const onNodesChange: OnNodesChange<Node> = useCallback(
    (changes) => {
      setNodes((nds) => applyNodeChanges(changes, nds));

      // Handle position changes (use Set for O(1) lookup instead of Array.find)
      changes.forEach((change) => {
        if (change.type === 'position' && 'position' in change && change.position && !change.dragging) {
          if (fileNodeIds.has(change.id) && change.position) {
            updateNodePosition(change.id, change.position.x, change.position.y);
          }
        }
      });
    },
    [fileNodeIds, updateNodePosition, setNodes]
  );

  const onSelectionChange = useCallback(
    ({ nodes: selectedNodes }: { nodes: Node[] }) => {
      setSelectedNodeIds(selectedNodes.map((n) => n.id));
    },
    [setSelectedNodeIds]
  );

  // Loading state: show spinner while a project is loading.
  if (currentProject && isLoading) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50 dark:bg-gray-900">
        <div className="text-center">
          <div className="inline-block w-10 h-10 border-4 border-blue-500 border-t-transparent rounded-full animate-spin mb-3"></div>
          <p className="text-gray-500 dark:text-gray-400">加载项目中…</p>
        </div>
      </div>
    );
  }

  if (!currentProject) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50 dark:bg-gray-900">
        <div className="text-center text-gray-500 dark:text-gray-400">
          <div className="text-4xl mb-4">📁</div>
          <p className="text-lg">请从侧边栏选择或创建一个项目</p>
        </div>
      </div>
    );
  }

  // Handle drag over event
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'copy';
    setIsDragging(true);
  }, []);

  // Handle drop event - add files from system file manager
  const onDrop = useCallback(async (event: React.DragEvent) => {
    event.preventDefault();
    setIsDragging(false);

    // Get the dropped file paths
    const files = event.dataTransfer.files;
    if (files.length === 0) return;

    if (!currentProject) {
      toast.error('请先创建一个项目');
      return;
    }

    // Get drop position in canvas coordinates
    const position = screenToFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });

    // Process each dropped file
    const newNodes: FileNode[] = [];

    for (let i = 0; i < files.length; i++) {
      const file = files[i] as File & { path?: string };
      const filePath = file.path;

      if (!filePath) continue;

      // Calculate position with offset for multiple files
      const offsetX = (i % 5) * 200;
      const offsetY = Math.floor(i / 5) * 150;

      // Check if file/folder already exists in project
      const existingNode = fileNodes.find(n => n.path === filePath);
      if (existingNode) continue;

      // Create new file node
      const newNode: FileNode = {
        id: `node-${Date.now()}-${i}`,
        projectId: currentProject.id,
        path: filePath,
        name: file.name,
        extension: file.name.includes('.') ? file.name.split('.').pop() || '' : '',
        size: file.size || 0,
        createdAt: new Date().toISOString(),
        modifiedAt: new Date().toISOString(),
        tags: [],
        parentId: null,
        positionX: position.x + offsetX,
        positionY: position.y + offsetY,
        isCollapsed: false,
        isDirectory: file.type === 'directory',
        children: [],
        relatedFiles: [],
      };

      newNodes.push(newNode);
    }

    if (newNodes.length > 0) {
      // Add new nodes to store
      setFileNodes([...fileNodes, ...newNodes]);
      console.log(`已添加 ${newNodes.length} 个文件到画布`);
    }
  }, [currentProject, fileNodes, screenToFlowPosition, setFileNodes]);

  return (
    <div className="h-full w-full">
      {/* Drop overlay */}
      {isDragging && (
        <div className="absolute inset-0 bg-blue-500/20 border-2 border-dashed border-blue-500 z-50 flex items-center justify-center pointer-events-none">
          <div className="bg-white px-6 py-4 rounded-lg shadow-lg">
            <p className="text-blue-600 font-medium">拖放文件到此处</p>
          </div>
        </div>
      )}
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onDragOver={onDragOver}
        onDrop={onDrop}
        nodeTypes={nodeTypes}
        snapToGrid
        snapGrid={[15, 15]}
        defaultViewport={viewport}
        onMoveEnd={(_, vp) => setViewport(vp)}
        onSelectionChange={onSelectionChange}
        proOptions={{ hideAttribution: true }}
      >
        <Background color={darkMode ? '#374151' : '#e2e8f0'} gap={20} />
        <Controls className={`shadow-md rounded-lg ${darkMode ? 'bg-gray-800' : 'bg-white'}`} />
        <MiniMap
          className={`shadow-md rounded-lg ${darkMode ? 'bg-gray-800' : 'bg-white'}`}
          nodeColor={darkMode ? '#60a5fa' : '#3b82f6'}
          maskColor={darkMode ? 'rgba(0, 0, 0, 0.4)' : 'rgba(0, 0, 0, 0.1)'}
          style={darkMode ? { border: '1px solid #4b5563' } : {}}
        />
      </ReactFlow>

      {/* Batch Operations Toolbar */}
      <BatchToolbar />
    </div>
  );
}

// 导出的主组件
export default function Canvas() {
  return (
    <ReactFlowProvider>
      <CanvasInner />
    </ReactFlowProvider>
  );
}
