import { useState, useEffect, useRef, useCallback } from 'react';
import Editor from '@monaco-editor/react';
import { useAppStore } from '../../stores/appStore';
import * as api from '../../services/api';
import { convertFileSrc } from '@tauri-apps/api/core';
import toast from 'react-hot-toast';

const IMAGE_EXTS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico'];
const VIDEO_EXTS = ['mp4', 'webm', 'mkv', 'mov'];
const AUDIO_EXTS = ['mp3', 'wav', 'ogg', 'm4a'];

/** Debounce interval for auto-save (ms) */
const AUTO_SAVE_DELAY = 2000;

export default function ViewerPanel() {
  const { openTabs, activeTabId, updateTab, fileNodes } = useAppStore();
  const [content, setContent] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showHistory, setShowHistory] = useState(false);
  const [history, setHistory] = useState<api.FileEditHistory[]>([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const initialContentRef = useRef<string>('');
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const activeTab = openTabs.find((t) => t.id === activeTabId);

  // Load file content when switching tabs — restore from store if available
  useEffect(() => {
    if (!activeTab) {
      setContent('');
      return;
    }

    // If the tab already has cached content, use it (no re-fetch needed)
    if (activeTab.content !== undefined) {
      setContent(activeTab.content);
      return;
    }

    const loadContent = async () => {
      setLoading(true);
      setError(null);

      try {
        const result = await api.readFileContent(activeTab.path);
        setContent(result.content);
        initialContentRef.current = result.content;
        // Cache content in the store so we don't re-fetch on tab switch
        updateTab(activeTab.id, { content: result.content });
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load file');
        setContent('');
      } finally {
        setLoading(false);
      }
    };

    loadContent();
  }, [activeTab?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSave = useCallback(async () => {
    if (!activeTab) return;

    // Cancel any pending auto-save
    if (autoSaveTimerRef.current) {
      clearTimeout(autoSaveTimerRef.current);
    }

    try {
      // P1-8: Detect external file changes before overwriting.
      if (lastMtimeRef.current) {
        try {
          const current = await api.readFileContent(activeTab.path);
          const currentMtime = current.modifiedAt ?? null;
          if (currentMtime !== null && currentMtime !== lastMtimeRef.current) {
            const overwrite = confirm(
              '文件已被外部程序修改。是否覆盖外部修改？\n\n点击“确定”覆盖，点击“取消”重新加载外部版本。'
            );
            if (!overwrite) {
              setContent(current.content);
              initialContentRef.current = current.content;
              lastMtimeRef.current = currentMtime;
              updateTab(activeTab.id, { content: current.content, isModified: false });
              return;
            }
          }
        } catch {
          // If we cannot read the file (e.g. deleted), just proceed with save
        }
      }

      const fileNode = fileNodes.find(n => n.path === activeTab.path);
      await api.writeFileContent(activeTab.path, content, fileNode?.id, fileNode?.projectId);
      initialContentRef.current = content;
      // Refresh mtime after successful save
      try {
        const fresh = await api.readFileContent(activeTab.path);
        lastMtimeRef.current = fresh.modifiedAt ?? null;
      } catch { /* ignore */ }
      updateTab(activeTab.id, { isModified: false });
    } catch (err) {
      toast.error('保存失败: ' + (err instanceof Error ? err.message : String(err)));
    }
  }, [activeTab, content, fileNodes, updateTab]);

  // Ctrl+S / Cmd+S keyboard shortcut
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (activeTab?.isModified) {
          handleSave();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeTab?.isModified, handleSave]);

  // Auto-save with debounce
  const scheduleAutoSave = useCallback(() => {
    if (autoSaveTimerRef.current) {
      clearTimeout(autoSaveTimerRef.current);
    }
    autoSaveTimerRef.current = setTimeout(() => {
      if (activeTab?.isModified) {
        handleSave();
      }
    }, AUTO_SAVE_DELAY);
  }, [activeTab, handleSave]);

  const handleEditorChange = (value: string | undefined) => {
    if (!activeTab || value === undefined) return;
    setContent(value);
    const isModified = value !== initialContentRef.current;
    // Persist content to store immediately so switching tabs doesn't lose edits
    updateTab(activeTab.id, { content: value, isModified });
    // Schedule auto-save
    if (isModified) {
      scheduleAutoSave();
    }
  };

  const loadHistory = async () => {
    const fileNode = fileNodes.find(n => n.path === activeTab?.path);
    if (!fileNode) return;
    
    setLoadingHistory(true);
    try {
      const hist = await api.getFileHistory(fileNode.id);
      setHistory(hist);
    } catch (err) {
      console.error('Failed to load history:', err);
    } finally {
      setLoadingHistory(false);
    }
  };

  const handleShowHistory = () => {
    if (!showHistory) {
      loadHistory();
    }
    setShowHistory(!showHistory);
  };

  const handleRestore = async (versionId: string) => {
    if (!confirm('确定要恢复到这个版本吗？当前修改将被覆盖。')) return;
    if (!activeTab) return;
    
    try {
      const restoredContent = await api.restoreFileVersion(versionId);
      setContent(restoredContent);
      initialContentRef.current = restoredContent;
      // Persist restored content and immediately save
      updateTab(activeTab.id, { content: restoredContent, isModified: true });
      setShowHistory(false);
      // Auto-save the restored version
      try {
        const fileNode = fileNodes.find(n => n.path === activeTab.path);
        await api.writeFileContent(activeTab.path, restoredContent, fileNode?.id, fileNode?.projectId);
        updateTab(activeTab.id, { isModified: false });
      } catch (saveErr) {
        console.warn('Auto-save after restore failed:', saveErr);
      }
    } catch (err) {
      toast.error('恢复失败: ' + (err instanceof Error ? err.message : String(err)));
    }
  };

  // Cleanup auto-save timer on unmount
  useEffect(() => {
    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current);
      }
    };
  }, []);

  const fileExt = activeTab?.name.split('.').pop()?.toLowerCase() || '';
  const isImage = IMAGE_EXTS.includes(fileExt);
  const isVideo = VIDEO_EXTS.includes(fileExt);
  const isAudio = AUDIO_EXTS.includes(fileExt);

  if (!activeTab) {
    return (
      <div className="h-full flex items-center justify-center text-gray-400">
        <p>选择一个文件查看</p>
      </div>
    );
  }

  const mediaSrc = convertFileSrc(activeTab.path);

  return (
    <div className="h-full flex flex-col bg-white">
      <div className="flex items-center justify-between px-4 py-2 bg-gray-50 border-b">
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-600">{activeTab.path}</span>
        </div>
        <div className="flex items-center gap-2">
          {activeTab.isModified && (
            <span className="text-xs text-orange-500">未保存</span>
          )}
          <button
            onClick={handleShowHistory}
            className="px-3 py-1 rounded text-sm bg-gray-200 hover:bg-gray-300 transition-colors"
          >
            📜 历史
          </button>
          <button
            onClick={handleSave}
            disabled={!activeTab.isModified}
            className={`
              px-3 py-1 rounded text-sm transition-colors
              ${activeTab.isModified 
                ? 'bg-blue-600 hover:bg-blue-700 text-white' 
                : 'bg-gray-200 text-gray-400 cursor-not-allowed'
              }
            `}
          >
            保存
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        {loading && (
          <div className="h-full flex items-center justify-center">
            <span className="animate-spin text-2xl">⏳</span>
          </div>
        )}

        {error && (
          <div className="h-full flex items-center justify-center text-red-500">
            <p>加载失败: {error}</p>
          </div>
        )}

        {!loading && !error && isImage && (
          <div className="h-full flex items-center justify-center p-4 bg-gray-100">
            <img
              src={mediaSrc}
              alt={activeTab.name}
              className="max-w-full max-h-full object-contain"
            />
          </div>
        )}

        {!loading && !error && isVideo && (
          <div className="h-full flex items-center justify-center bg-black">
            <video
              src={mediaSrc}
              controls
              className="max-w-full max-h-full"
            />
          </div>
        )}

        {!loading && !error && isAudio && (
          <div className="h-full flex items-center justify-center bg-gray-100">
            <audio
              src={mediaSrc}
              controls
              className="w-full max-w-md"
            />
          </div>
        )}

        {!loading && !error && !isImage && !isVideo && !isAudio && (
          <Editor
            height="100%"
            language={getLanguage(fileExt)}
            value={content}
            onChange={handleEditorChange}
            theme="vs-light"
            options={{
              readOnly: activeTab.type === 'viewer',
              minimap: { enabled: false },
              fontSize: 14,
              lineNumbers: 'on',
              wordWrap: 'on',
              automaticLayout: true,
            }}
          />
        )}
      </div>

      {/* History Panel */}
      {showHistory && (
        <div className="h-48 border-t bg-gray-50 flex flex-col">
          <div className="flex items-center justify-between px-4 py-2 border-b bg-white">
            <span className="font-medium text-sm">编辑历史</span>
            <button onClick={() => setShowHistory(false)} className="text-gray-500 hover:text-gray-700">✕</button>
          </div>
          <div className="flex-1 overflow-y-auto p-2">
            {loadingHistory ? (
              <div className="text-center text-gray-400 py-4">加载中...</div>
            ) : history.length === 0 ? (
              <div className="text-center text-gray-400 py-4">暂无编辑历史</div>
            ) : (
              <div className="space-y-2">
                {history.map((h, idx) => (
                  <div key={h.id} className="flex items-center justify-between bg-white p-2 rounded shadow-sm">
                    <div className="flex-1 min-w-0">
                      <div className="text-xs text-gray-500">
                        {new Date(h.createdAt).toLocaleString()}
                        {idx === 0 && <span className="ml-2 text-blue-500">(最新)</span>}
                      </div>
                      {h.diff && (
                        <pre className="text-xs text-gray-600 mt-1 whitespace-pre-wrap line-clamp-2 font-mono">
                          {h.diff.substring(0, 200)}...
                        </pre>
                      )}
                    </div>
                    <button
                      onClick={() => handleRestore(h.id)}
                      className="ml-2 px-2 py-1 text-xs bg-green-500 hover:bg-green-600 text-white rounded"
                    >
                      恢复
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function getLanguage(ext: string): string {
  const langMap: Record<string, string> = {
    js: 'javascript',
    jsx: 'javascript',
    ts: 'typescript',
    tsx: 'typescript',
    py: 'python',
    rs: 'rust',
    go: 'go',
    java: 'java',
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    hpp: 'cpp',
    html: 'html',
    css: 'css',
    scss: 'scss',
    less: 'less',
    json: 'json',
    xml: 'xml',
    yaml: 'yaml',
    yml: 'yaml',
    md: 'markdown',
    sql: 'sql',
    sh: 'shell',
    bash: 'shell',
    ps1: 'powershell',
    lua: 'lua',
    php: 'php',
    swift: 'swift',
    kt: 'kotlin',
  };
  return langMap[ext] || 'plaintext';
}
