import React, { useState } from 'react';
import { useAppStore } from '../../stores/appStore';
import { batchOperation, BatchResult } from '../../services/api';
import * as api from '../../services/api';
import toast from 'react-hot-toast';

interface BatchToolbarProps {
  onComplete?: (result: BatchResult) => void;
}

export const BatchToolbar: React.FC<BatchToolbarProps> = ({ onComplete }) => {
  const { selectedNodeIds, fileNodes, setFileNodes, setSelectedNodeIds } = useAppStore();
  const [isProcessing, setIsProcessing] = useState(false);
  const [showMoveDialog, setShowMoveDialog] = useState(false);
  const [showCopyDialog, setShowCopyDialog] = useState(false);

  const selectedFiles = fileNodes.filter(n => selectedNodeIds.includes(n.id));

  const handleBatchTrash = async () => {
    if (selectedNodeIds.length === 0) return;
    if (!confirm(`确定要将 ${selectedNodeIds.length} 个项目移到回收站吗？`)) return;

    setIsProcessing(true);
    try {
      const result = await batchOperation({
        operation: 'trash',
        paths: selectedFiles.map(f => f.path),
      });
      
      if (result.success || result.processed > 0) {
        // Remove from fileNodes
        setFileNodes(fileNodes.filter(n => !selectedNodeIds.includes(n.id)));
        setSelectedNodeIds([]);
        toast.success(`已移动 ${result.processed} 个项目到回收站`);
      } else if (result.failed > 0) {
        toast.error(`操作完成：${result.processed} 成功，${result.failed} 失败\n${result.errors.join('\n')}`);
      }
      onComplete?.(result);
    } catch (error) {
      console.error('Batch trash failed:', error);
      toast.error('操作失败');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleBatchDelete = async () => {
    if (selectedNodeIds.length === 0) return;
    if (!confirm(`确定要永久删除 ${selectedNodeIds.length} 个项目吗？此操作不可恢复！`)) return;

    setIsProcessing(true);
    try {
      const result = await batchOperation({
        operation: 'delete',
        paths: selectedFiles.map(f => f.path),
      });
      
      if (result.success || result.processed > 0) {
        setFileNodes(fileNodes.filter(n => !selectedNodeIds.includes(n.id)));
        setSelectedNodeIds([]);
        toast.success(`已永久删除 ${result.processed} 个项目`);
      } else if (result.failed > 0) {
        toast.error(`操作完成：${result.processed} 成功，${result.failed} 失败\n${result.errors.join('\n')}`);
      }
      onComplete?.(result);
    } catch (error) {
      console.error('Batch delete failed:', error);
      toast.error('操作失败');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleBatchMove = async () => {
    setShowMoveDialog(true);
  };

  const handleBatchCopy = async () => {
    setShowCopyDialog(true);
  };

  const confirmMove = async (destPath: string) => {
    setIsProcessing(true);
    setShowMoveDialog(false);
    try {
      const result = await batchOperation({
        operation: 'move',
        paths: selectedFiles.map(f => f.path),
        destination: destPath,
      });
      
      if (result.success || result.processed > 0) {
        // Refresh file list
        toast.success(`已移动 ${result.processed} 个项目`);
        setSelectedNodeIds([]);
      } else {
        toast.error(`操作完成：${result.processed} 成功，${result.failed} 失败\n${result.errors.join('\n')}`);
      }
      onComplete?.(result);
    } catch (error) {
      console.error('Batch move failed:', error);
      toast.error('操作失败');
    } finally {
      setIsProcessing(false);
    }
  };

  const confirmCopy = async (destPath: string) => {
    setIsProcessing(true);
    setShowCopyDialog(false);
    try {
      const result = await batchOperation({
        operation: 'copy',
        paths: selectedFiles.map(f => f.path),
        destination: destPath,
      });
      
      if (result.success || result.processed > 0) {
        toast.success(`已复制 ${result.processed} 个项目`);
        setSelectedNodeIds([]);
      } else {
        toast.error(`操作完成：${result.processed} 成功，${result.failed} 失败\n${result.errors.join('\n')}`);
      }
      onComplete?.(result);
    } catch (error) {
      console.error('Batch copy failed:', error);
      toast.error('操作失败');
    } finally {
      setIsProcessing(false);
    }
  };

  if (selectedNodeIds.length === 0) {
    return null;
  }

  return (
    <>
      <div className="fixed bottom-4 left-1/2 transform -translate-x-1/2 bg-gray-900 text-white px-4 py-2 rounded-lg shadow-lg flex items-center gap-3 z-50">
        <span className="text-sm font-medium">
          已选择 <span className="text-blue-400">{selectedNodeIds.length}</span> 项
        </span>
        
        <div className="h-4 w-px bg-gray-600" />
        
        <button
          onClick={handleBatchMove}
          disabled={isProcessing}
          className="px-3 py-1 text-sm bg-blue-600 hover:bg-blue-700 rounded disabled:opacity-50"
        >
          📦 移动到...
        </button>
        
        <button
          onClick={handleBatchCopy}
          disabled={isProcessing}
          className="px-3 py-1 text-sm bg-green-600 hover:bg-green-700 rounded disabled:opacity-50"
        >
          📑 复制到...
        </button>
        
        <button
          onClick={handleBatchTrash}
          disabled={isProcessing}
          className="px-3 py-1 text-sm bg-yellow-600 hover:bg-yellow-700 rounded disabled:opacity-50"
        >
          🗑️ 移到回收站
        </button>
        
        <button
          onClick={handleBatchDelete}
          disabled={isProcessing}
          className="px-3 py-1 text-sm bg-red-600 hover:bg-red-700 rounded disabled:opacity-50"
        >
          ❌ 永久删除
        </button>
        
        <button
          onClick={() => setSelectedNodeIds([])}
          className="px-2 py-1 text-sm bg-gray-600 hover:bg-gray-500 rounded"
        >
          ✕ 取消
        </button>

        {isProcessing && <span className="text-sm">⏳ 处理中...</span>}
      </div>

      {/* Move Dialog */}
      {showMoveDialog && (
        <DirectoryDialog
          title="移动到"
          onConfirm={confirmMove}
          onCancel={() => setShowMoveDialog(false)}
        />
      )}

      {/* Copy Dialog */}
      {showCopyDialog && (
        <DirectoryDialog
          title="复制到"
          onConfirm={confirmCopy}
          onCancel={() => setShowCopyDialog(false)}
        />
      )}
    </>
  );
};

// Simple directory selection dialog
interface DirectoryDialogProps {
  title: string;
  onConfirm: (path: string) => void;
  onCancel: () => void;
}

const DirectoryDialog: React.FC<DirectoryDialogProps> = ({ title, onConfirm, onCancel }) => {
  const [path, setPath] = useState('');

  const handleBrowse = async () => {
    try {
      const selected = await api.openDirectoryDialog();
      if (selected) {
        setPath(selected);
      }
    } catch (error) {
      console.error('Failed to open dialog:', error);
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-4 w-96 shadow-xl">
        <h3 className="font-bold text-lg mb-3">{title}</h3>
        
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            placeholder="选择目标文件夹..."
            className="flex-1 px-3 py-2 border rounded text-sm dark:bg-gray-700 dark:border-gray-600"
          />
          <button
            onClick={handleBrowse}
            className="px-3 py-2 bg-gray-200 dark:bg-gray-600 rounded text-sm"
          >
            浏览
          </button>
        </div>
        
        <div className="flex gap-2 justify-end">
          <button
            onClick={onCancel}
            className="px-4 py-2 bg-gray-200 dark:bg-gray-600 rounded text-sm"
          >
            取消
          </button>
          <button
            onClick={() => onConfirm(path)}
            disabled={!path}
            className="px-4 py-2 bg-blue-600 text-white rounded text-sm disabled:opacity-50"
          >
            确定
          </button>
        </div>
      </div>
    </div>
  );
};
