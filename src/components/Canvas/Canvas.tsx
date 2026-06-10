import { useCallback, useEffect } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  Node,
  Edge,
  applyNodeChanges,
  OnNodesChange,
  OnEdgesChange,
  OnConnect,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useAppStore } from '../../stores/appStore';
import FileNodeComponent from './FileNode';

const nodeTypes = {
  file: FileNodeComponent,
};

export default function Canvas() {
  const { 
    fileNodes, 
    relations,
    currentProject,
    viewport,
    setViewport,
    selectedNodeIds,
    setSelectedNodeIds,
    updateNodePosition,
  } = useAppStore();

  // Convert fileNodes to React Flow nodes
  const initialNodes: Node[] = fileNodes.map((node) => ({
    id: node.id,
    type: 'file',
    position: { x: node.positionX, y: node.positionY },
    data: { ...node },
  }));

  // Convert relations to React Flow edges
  const initialEdges: Edge[] = relations.map((rel) => ({
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
  }));

  const [nodes, setNodes] = useNodesState(initialNodes);
  const [edges, setEdges] = useEdgesState(initialEdges);

  // Update nodes when fileNodes change
  useEffect(() => {
    setNodes(initialNodes);
  }, [fileNodes, selectedNodeIds, setNodes]);

  // Update edges when relations change
  useEffect(() => {
    setEdges(initialEdges);
  }, [relations, setEdges]);

  const onNodesChange: OnNodesChange<Node> = useCallback(
    (changes) => {
      setNodes((nds) => applyNodeChanges(changes, nds));
      
      // Handle position changes
      changes.forEach((change) => {
        if (change.type === 'position' && 'position' in change && change.position && !change.dragging) {
          const node = fileNodes.find((n) => n.id === change.id);
          if (node && 'position' in change && change.position) {
            updateNodePosition(change.id, change.position.x, change.position.y);
          }
        }
      });
    },
    [fileNodes, updateNodePosition, setNodes]
  );

  const onEdgesChange: OnEdgesChange<Edge> = useCallback(
    () => {
      // Handle edge changes if needed
    },
    []
  );

  const onConnect: OnConnect = useCallback(
    () => {
      // Handle connection if needed
    },
    []
  );

  const onSelectionChange = useCallback(
    ({ nodes: selectedNodes }: { nodes: Node[] }) => {
      setSelectedNodeIds(selectedNodes.map((n) => n.id));
    },
    [setSelectedNodeIds]
  );

  if (!currentProject) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-50">
        <div className="text-center text-gray-500">
          <div className="text-4xl mb-4">📁</div>
          <p className="text-lg">请从侧边栏选择或创建一个项目</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onSelectionChange={onSelectionChange}
        nodeTypes={nodeTypes}
        fitView
        snapToGrid
        snapGrid={[15, 15]}
        defaultViewport={viewport}
        onMoveEnd={(_, vp) => setViewport(vp)}
        proOptions={{ hideAttribution: true }}
      >
        <Background color="#e2e8f0" gap={20} />
        <Controls className="bg-white shadow-md rounded-lg" />
        <MiniMap 
          className="bg-white shadow-md rounded-lg"
          nodeColor="#3b82f6"
          maskColor="rgba(0, 0, 0, 0.1)"
        />
      </ReactFlow>
    </div>
  );
}
