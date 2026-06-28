import React, { useMemo } from 'react';
import { useAppStore } from '../../stores/appStore';

interface StatsPanelProps {}

export const StatsPanel: React.FC<StatsPanelProps> = () => {
  const { fileNodes, currentProject } = useAppStore();

  const stats = useMemo(() => {
    const totalFiles = fileNodes.filter(n => !n.isDirectory).length;
    const totalDirs = fileNodes.filter(n => n.isDirectory).length;
    const totalSize = fileNodes.reduce((sum, n) => sum + n.size, 0);
    
    // Size by type
    const sizeByType: Record<string, number> = {};
    const countByType: Record<string, number> = {};
    fileNodes.forEach(n => {
      const ext = n.isDirectory ? 'folder' : (n.extension || 'no-ext');
      sizeByType[ext] = (sizeByType[ext] || 0) + n.size;
      countByType[ext] = (countByType[ext] || 0) + 1;
    });

    // Top extensions by count
    const topTypes = Object.entries(countByType)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8);

    // Tag distribution
    const tagCount: Record<string, number> = {};
    fileNodes.forEach(n => {
      n.tags.forEach(tag => {
        tagCount[tag] = (tagCount[tag] || 0) + 1;
      });
    });
    const topTags = Object.entries(tagCount)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6);

    // Size distribution
    const sizeRanges = [
      { label: '< 1KB', min: 0, max: 1024, count: 0 },
      { label: '1KB-100KB', min: 1024, max: 100 * 1024, count: 0 },
      { label: '100KB-1MB', min: 100 * 1024, max: 1024 * 1024, count: 0 },
      { label: '1MB-10MB', min: 1024 * 1024, max: 10 * 1024 * 1024, count: 0 },
      { label: '> 10MB', min: 10 * 1024 * 1024, max: Infinity, count: 0 },
    ];
    fileNodes.forEach(n => {
      const range = sizeRanges.find(r => n.size >= r.min && n.size < r.max);
      if (range) range.count++;
    });

    return {
      totalFiles,
      totalDirs,
      totalSize,
      topTypes,
      topTags,
      sizeRanges,
    };
  }, [fileNodes]);

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  if (!currentProject) {
    return (
      <div className="p-4 text-center text-gray-500">
        请先选择一个项目
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto bg-white dark:bg-gray-800 p-4">
      <h3 className="font-bold text-lg mb-4 flex items-center gap-2">
        <span>📊</span>
        <span>项目统计</span>
      </h3>

      {/* Overview Cards */}
      <div className="grid grid-cols-2 gap-3 mb-4">
        <div className="bg-blue-50 dark:bg-blue-900 p-3 rounded-lg">
          <div className="text-2xl font-bold text-blue-600 dark:text-blue-300">
            {stats.totalFiles}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">文件</div>
        </div>
        <div className="bg-green-50 dark:bg-green-900 p-3 rounded-lg">
          <div className="text-2xl font-bold text-green-600 dark:text-green-300">
            {stats.totalDirs}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">文件夹</div>
        </div>
        <div className="col-span-2 bg-purple-50 dark:bg-purple-900 p-3 rounded-lg">
          <div className="text-2xl font-bold text-purple-600 dark:text-purple-300">
            {formatSize(stats.totalSize)}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">总大小</div>
        </div>
      </div>

      {/* File Types Chart */}
      <div className="mb-4">
        <h4 className="font-semibold text-sm mb-2 flex items-center gap-1">
          <span>📁</span>
          文件类型分布
        </h4>
        <div className="space-y-2">
          {stats.topTypes.map(([ext, count]) => {
            const percent = stats.totalFiles > 0 ? (count / stats.totalFiles) * 100 : 0;
            const colors = ['bg-blue-500', 'bg-green-500', 'bg-yellow-500', 'bg-red-500', 'bg-purple-500', 'bg-pink-500', 'bg-indigo-500', 'bg-gray-500'];
            const color = colors[stats.topTypes.indexOf([ext, count]) % colors.length];
            
            return (
              <div key={ext} className="flex items-center gap-2">
                <span className="text-xs w-16 truncate" title={ext}>{ext}</span>
                <div className="flex-1 h-4 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                  <div 
                    className={`h-full ${color} transition-all`}
                    style={{ width: `${percent}%` }}
                  />
                </div>
                <span className="text-xs text-gray-500 w-8 text-right">{count}</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Size Distribution */}
      <div className="mb-4">
        <h4 className="font-semibold text-sm mb-2 flex items-center gap-1">
          <span>📏</span>
          大小分布
        </h4>
        <div className="space-y-1">
          {stats.sizeRanges.map((range, idx) => {
            const maxCount = Math.max(...stats.sizeRanges.map(r => r.count), 1);
            const percent = (range.count / maxCount) * 100;
            
            return (
              <div key={idx} className="flex items-center gap-2">
                <span className="text-xs w-20">{range.label}</span>
                <div className="flex-1 h-3 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                  <div 
                    className="h-full bg-teal-500 transition-all"
                    style={{ width: `${percent}%` }}
                  />
                </div>
                <span className="text-xs text-gray-500 w-6 text-right">{range.count}</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Tags Distribution */}
      {stats.topTags.length > 0 && (
        <div className="mb-4">
          <h4 className="font-semibold text-sm mb-2 flex items-center gap-1">
            <span>🏷️</span>
            标签分布
          </h4>
          <div className="flex flex-wrap gap-1">
            {stats.topTags.map(([tag, count]) => (
              <span 
                key={tag}
                className="inline-flex items-center gap-1 px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-xs"
              >
                <span className="w-2 h-2 bg-blue-400 rounded-full"></span>
                {tag}
                <span className="text-gray-500">({count})</span>
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Summary */}
      <div className="text-xs text-gray-500 border-t pt-3">
        <p>共 {fileNodes.length} 个项目</p>
        <p>更新于 {new Date().toLocaleTimeString()}</p>
      </div>
    </div>
  );
};