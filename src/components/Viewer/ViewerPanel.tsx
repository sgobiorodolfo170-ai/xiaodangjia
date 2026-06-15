import { useState, useEffect, useRef } from 'react';
import Editor from '@monaco-editor/react';
import { useAppStore } from '../../stores/appStore';
import * as api from '../../services/api';
import { convertFileSrc } from '@tauri-apps/api/core';

const IMAGE_EXTS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico'];
const VIDEO_EXTS = ['mp4', 'webm', 'mkv', 'mov'];
const AUDIO_EXTS = ['mp3', 'wav', 'ogg', 'm4a'];

export default function ViewerPanel() {
  const { openTabs, activeTabId, updateTab } = useAppStore();
  const [content, setContent] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const initialContentRef = useRef<string>('');

  const activeTab = openTabs.find((t) => t.id === activeTabId);

  useEffect(() => {
    if (!activeTab) {
      setContent('');
      return;
    }

    const loadContent = async () => {
      setLoading(true);
      setError(null);

      try {
        const result = await api.readFileContent(activeTab.path);
        setContent(result.content);
        initialContentRef.current = result.content;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load file');
        setContent('');
      } finally {
        setLoading(false);
      }
    };

    loadContent();
  }, [activeTab?.path]);

  const handleEditorChange = (value: string | undefined) => {
    if (!activeTab || value === undefined) return;
    setContent(value);
    const isModified = value !== initialContentRef.current;
    updateTab(activeTab.id, { isModified });
  };

  const handleSave = async () => {
    if (!activeTab) return;

    try {
      await api.writeFileContent(activeTab.path, content);
      initialContentRef.current = content;
      updateTab(activeTab.id, { isModified: false });
    } catch (err) {
      alert('Failed to save: ' + (err instanceof Error ? err.message : err));
    }
  };

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
