import React, { useState } from 'react';
import { FileNode } from '../../types';
import { useAppStore } from '../../stores/appStore';
import { advancedSearch, SearchFilters } from '../../services/api';

interface SearchFilterPanelProps {
  onResults: (results: FileNode[]) => void;
}

// Common file type options
const FILE_TYPES = [
  { value: 'js', label: 'JavaScript' },
  { value: 'ts', label: 'TypeScript' },
  { value: 'tsx', label: 'TSX' },
  { value: 'jsx', label: 'JSX' },
  { value: 'py', label: 'Python' },
  { value: 'rs', label: 'Rust' },
  { value: 'go', label: 'Go' },
  { value: 'java', label: 'Java' },
  { value: 'json', label: 'JSON' },
  { value: 'md', label: 'Markdown' },
  { value: 'txt', label: 'Text' },
  { value: 'html', label: 'HTML' },
  { value: 'css', label: 'CSS' },
  { value: 'png', label: 'PNG Image' },
  { value: 'jpg', label: 'JPG Image' },
  { value: 'svg', label: 'SVG' },
  { value: 'zip', label: 'ZIP' },
  { value: 'folder', label: 'Folder' },
];

// Size presets (in bytes)
const SIZE_PRESETS = [
  { label: 'Any', min: undefined, max: undefined },
  { label: '< 1KB', min: 0, max: 1024 },
  { label: '< 100KB', min: 0, max: 100 * 1024 },
  { label: '< 1MB', min: 0, max: 1024 * 1024 },
  { label: '< 10MB', min: 0, max: 10 * 1024 * 1024 },
  { label: '> 10MB', min: 10 * 1024 * 1024, max: undefined },
];

// Date presets
const DATE_PRESETS = [
  { label: 'Any', after: undefined, before: undefined },
  { label: 'Today', after: getDateOffset(0), before: undefined },
  { label: 'This Week', after: getDateOffset(7), before: undefined },
  { label: 'This Month', after: getDateOffset(30), before: undefined },
  { label: 'This Year', after: getDateOffset(365), before: undefined },
];

function getDateOffset(days: number): string {
  const date = new Date();
  date.setDate(date.getDate() - days);
  return date.toISOString();
}

export const SearchFilterPanel: React.FC<SearchFilterPanelProps> = ({ onResults }) => {
  const { currentProject } = useAppStore();
  
  const [query, setQuery] = useState('');
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [sizePreset, setSizePreset] = useState(0);
  const [datePreset, setDatePreset] = useState(0);
  const [showDirectoryOnly, setShowDirectoryOnly] = useState<boolean | null>(null);
  const [isSearching, setIsSearching] = useState(false);

  const handleTypeToggle = (type: string) => {
    setSelectedTypes(prev => 
      prev.includes(type) 
        ? prev.filter(t => t !== type)
        : [...prev, type]
    );
  };

  const handleSearch = async () => {
    if (!currentProject) return;

    setIsSearching(true);
    try {
      const size = SIZE_PRESETS[sizePreset];
      const date = DATE_PRESETS[datePreset];
      
      const filters: SearchFilters = {
        query: query.trim() || undefined,
        fileTypes: selectedTypes.length > 0 ? selectedTypes : undefined,
        minSize: size.min,
        maxSize: size.max,
        modifiedAfter: date.after,
        modifiedBefore: date.before,
        isDirectory: showDirectoryOnly ?? undefined,
      };

      const results = await advancedSearch(currentProject.id, filters);
      onResults(results);
    } catch (error) {
      console.error('Search failed:', error);
    } finally {
      setIsSearching(false);
    }
  };

  const handleClear = () => {
    setQuery('');
    setSelectedTypes([]);
    setSizePreset(0);
    setDatePreset(0);
    setShowDirectoryOnly(null);
    onResults([]);
  };

  return (
    <div className="p-3 bg-gray-800 text-white">
      <h3 className="font-bold mb-3 flex items-center gap-2">
        <span>🔍</span>
        <span>高级搜索</span>
      </h3>

      {/* Text Query */}
      <div className="mb-3">
        <label className="block text-xs text-gray-400 mb-1">关键词</label>
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索文件名或路径..."
          className="w-full px-2 py-1.5 bg-gray-700 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
        />
      </div>

      {/* File Types */}
      <div className="mb-3">
        <label className="block text-xs text-gray-400 mb-1">文件类型</label>
        <div className="flex flex-wrap gap-1 max-h-24 overflow-y-auto p-1 bg-gray-700 rounded">
          {FILE_TYPES.map(type => (
            <button
              key={type.value}
              onClick={() => handleTypeToggle(type.value)}
              className={`px-2 py-0.5 text-xs rounded transition-colors ${
                selectedTypes.includes(type.value)
                  ? 'bg-blue-500 text-white'
                  : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
              }`}
            >
              {type.label}
            </button>
          ))}
        </div>
        {selectedTypes.length > 0 && (
          <button
            onClick={() => setSelectedTypes([])}
            className="text-xs text-blue-400 hover:text-blue-300 mt-1"
          >
            清除类型
          </button>
        )}
      </div>

      {/* Size Filter */}
      <div className="mb-3">
        <label className="block text-xs text-gray-400 mb-1">文件大小</label>
        <select
          value={sizePreset}
          onChange={(e) => setSizePreset(Number(e.target.value))}
          className="w-full px-2 py-1.5 bg-gray-700 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
        >
          {SIZE_PRESETS.map((preset, idx) => (
            <option key={idx} value={idx}>{preset.label}</option>
          ))}
        </select>
      </div>

      {/* Date Filter */}
      <div className="mb-3">
        <label className="block text-xs text-gray-400 mb-1">修改时间</label>
        <select
          value={datePreset}
          onChange={(e) => setDatePreset(Number(e.target.value))}
          className="w-full px-2 py-1.5 bg-gray-700 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
        >
          {DATE_PRESETS.map((preset, idx) => (
            <option key={idx} value={idx}>{preset.label}</option>
          ))}
        </select>
      </div>

      {/* Directory/File Filter */}
      <div className="mb-3">
        <label className="block text-xs text-gray-400 mb-1">类型</label>
        <div className="flex gap-2">
          <button
            onClick={() => setShowDirectoryOnly(null)}
            className={`flex-1 px-2 py-1.5 text-xs rounded ${
              showDirectoryOnly === null
                ? 'bg-blue-500 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            全部
          </button>
          <button
            onClick={() => setShowDirectoryOnly(true)}
            className={`flex-1 px-2 py-1.5 text-xs rounded ${
              showDirectoryOnly === true
                ? 'bg-blue-500 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            📁 文件夹
          </button>
          <button
            onClick={() => setShowDirectoryOnly(false)}
            className={`flex-1 px-2 py-1.5 text-xs rounded ${
              showDirectoryOnly === false
                ? 'bg-blue-500 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            📄 文件
          </button>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-2">
        <button
          onClick={handleSearch}
          disabled={isSearching}
          className="flex-1 px-3 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm font-medium flex items-center justify-center gap-2"
        >
          {isSearching ? '⏳' : '🔍'}
          {isSearching ? '搜索中...' : '搜索'}
        </button>
        <button
          onClick={handleClear}
          className="px-3 py-2 bg-gray-600 hover:bg-gray-500 rounded text-sm"
        >
          ✕
        </button>
      </div>
    </div>
  );
};