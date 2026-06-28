import React, { useState, useEffect } from 'react';
import { useAppStore } from '../../stores/appStore';
import { 
  getFavorites, 
  addFavorite, 
  removeFavorite,
  listTags,
  createTag,
  deleteTag,
  addFileTag,
  removeFileTag,
  getFileTags 
} from '../../services/api';

interface FavoritesTagsPanelProps {
  onFileSelect?: (fileId: string) => void;
}

// Predefined tag colors
const TAG_COLORS = [
  '#ef4444', // red
  '#f97316', // orange
  '#eab308', // yellow
  '#22c55e', // green
  '#14b8a6', // teal
  '#3b82f6', // blue
  '#8b5cf6', // violet
  '#ec4899', // pink
  '#6b7280', // gray
];

export const FavoritesTagsPanel: React.FC<FavoritesTagsPanelProps> = ({ onFileSelect }) => {
  const { 
    currentProject, 
    fileNodes,
    favoriteIds, 
    setFavoriteIds, 
    addFavoriteId, 
    removeFavoriteId,
    customTags,
    setCustomTags,
    addCustomTag,
    removeCustomTag,
    fileCustomTags,
    setFileCustomTags,
    selectedNodeIds
  } = useAppStore();

  const [activeTab, setActiveTab] = useState<'favorites' | 'tags'>('favorites');
  const [showCreateTag, setShowCreateTag] = useState(false);
  const [newTagName, setNewTagName] = useState('');
  const [newTagColor, setNewTagColor] = useState(TAG_COLORS[0]);
  const [showTagMenuFor, setShowTagMenuFor] = useState<string | null>(null);

  // Load favorites when project changes
  useEffect(() => {
    if (currentProject?.id) {
      loadFavorites();
      loadTags();
    }
  }, [currentProject?.id]);

  const loadFavorites = async () => {
    if (!currentProject?.id) return;
    try {
      const favIds = await getFavorites(currentProject.id);
      setFavoriteIds(favIds);
    } catch (error) {
      console.error('Failed to load favorites:', error);
    }
  };

  const loadTags = async () => {
    try {
      const tags = await listTags();
      setCustomTags(tags);
    } catch (error) {
      console.error('Failed to load tags:', error);
    }
  };

  const handleToggleFavorite = async () => {
    if (!currentProject?.id || selectedNodeIds.length === 0) return;
    
    const fileId = selectedNodeIds[0];
    try {
      if (favoriteIds.includes(fileId)) {
        await removeFavorite(currentProject.id, fileId);
        removeFavoriteId(fileId);
      } else {
        await addFavorite(currentProject.id, fileId);
        addFavoriteId(fileId);
      }
    } catch (error) {
      console.error('Failed to toggle favorite:', error);
    }
  };

  const handleCreateTag = async () => {
    if (!newTagName.trim()) return;
    try {
      const tag = await createTag(newTagName.trim(), newTagColor);
      addCustomTag(tag);
      setNewTagName('');
      setShowCreateTag(false);
    } catch (error) {
      console.error('Failed to create tag:', error);
    }
  };

  const handleDeleteTag = async (tagId: string) => {
    try {
      await deleteTag(tagId);
      removeCustomTag(tagId);
    } catch (error) {
      console.error('Failed to delete tag:', error);
    }
  };

  const handleAddTagToFile = async (tagId: string) => {
    if (!currentProject?.id || selectedNodeIds.length === 0) return;
    
    const fileId = selectedNodeIds[0];
    try {
      await addFileTag(fileId, tagId);
      const tag = customTags.find(t => t.id === tagId);
      if (tag) {
        const currentTags = fileCustomTags[fileId] || [];
        setFileCustomTags(fileId, [...currentTags, tag]);
      }
      setShowTagMenuFor(null);
    } catch (error) {
      console.error('Failed to add tag to file:', error);
    }
  };

  const handleRemoveTagFromFile = async (tagId: string) => {
    if (selectedNodeIds.length === 0) return;
    
    const fileId = selectedNodeIds[0];
    try {
      await removeFileTag(fileId, tagId);
      const currentTags = fileCustomTags[fileId] || [];
      setFileCustomTags(fileId, currentTags.filter(t => t.id !== tagId));
    } catch (error) {
      console.error('Failed to remove tag from file:', error);
    }
  };

  // Get favorite nodes
  const favoriteNodes = fileNodes.filter(n => favoriteIds.includes(n.id));
  
  // Get current file's tags
  const currentFileTags = selectedNodeIds.length > 0 
    ? fileCustomTags[selectedNodeIds[0]] || []
    : [];

  // Load tags for current file when selection changes
  useEffect(() => {
    if (selectedNodeIds.length > 0) {
      const fileId = selectedNodeIds[0];
      getFileTags(fileId).then(tags => {
        setFileCustomTags(fileId, tags);
      }).catch(console.error);
    }
  }, [selectedNodeIds[0]]);

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-800">
      {/* Tab Headers */}
      <div className="flex border-b border-gray-200 dark:border-gray-700">
        <button
          className={`flex-1 px-4 py-2 text-sm font-medium ${
            activeTab === 'favorites' 
              ? 'text-blue-600 border-b-2 border-blue-600' 
              : 'text-gray-500 hover:text-gray-700'
          }`}
          onClick={() => setActiveTab('favorites')}
        >
          ⭐ 收藏
        </button>
        <button
          className={`flex-1 px-4 py-2 text-sm font-medium ${
            activeTab === 'tags' 
              ? 'text-blue-600 border-b-2 border-blue-600' 
              : 'text-gray-500 hover:text-gray-700'
          }`}
          onClick={() => setActiveTab('tags')}
        >
          🏷️ 标签
        </button>
      </div>

      {/* Favorites Tab */}
      {activeTab === 'favorites' && (
        <div className="flex-1 overflow-y-auto p-2">
          {selectedNodeIds.length > 0 && (
            <button
              onClick={handleToggleFavorite}
              className={`w-full mb-3 px-3 py-2 rounded text-sm font-medium flex items-center justify-center gap-2 ${
                favoriteIds.includes(selectedNodeIds[0])
                  ? 'bg-yellow-100 text-yellow-700 hover:bg-yellow-200'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {favoriteIds.includes(selectedNodeIds[0]) ? '⭐ 已收藏' : '☆ 添加收藏'}
            </button>
          )}
          
          {favoriteNodes.length === 0 ? (
            <div className="text-center text-gray-500 py-8">
              <p>暂无收藏</p>
              <p className="text-xs mt-1">选择文件后点击上方按钮添加</p>
            </div>
          ) : (
            <div className="space-y-1">
              {favoriteNodes.map(node => (
                <div
                  key={node.id}
                  onClick={() => onFileSelect?.(node.id)}
                  className="flex items-center gap-2 px-3 py-2 rounded hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer"
                >
                  <span className="text-yellow-500">⭐</span>
                  <span className="text-sm truncate flex-1" title={node.name}>
                    {node.name}
                  </span>
                  <span className="text-xs text-gray-400">
                    {node.isDirectory ? '📁' : '📄'}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Tags Tab */}
      {activeTab === 'tags' && (
        <div className="flex-1 overflow-y-auto p-2">
          {/* Tag Management */}
          <div className="mb-3">
            {!showCreateTag ? (
              <button
                onClick={() => setShowCreateTag(true)}
                className="w-full px-3 py-2 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                + 新建标签
              </button>
            ) : (
              <div className="p-2 bg-gray-50 dark:bg-gray-700 rounded">
                <input
                  type="text"
                  value={newTagName}
                  onChange={(e) => setNewTagName(e.target.value)}
                  placeholder="标签名称"
                  className="w-full px-2 py-1 text-sm border rounded mb-2"
                />
                <div className="flex gap-1 mb-2 flex-wrap">
                  {TAG_COLORS.map(color => (
                    <button
                      key={color}
                      onClick={() => setNewTagColor(color)}
                      className={`w-6 h-6 rounded-full ${newTagColor === color ? 'ring-2 ring-offset-1 ring-gray-400' : ''}`}
                      style={{ backgroundColor: color }}
                    />
                  ))}
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={handleCreateTag}
                    className="flex-1 px-2 py-1 text-sm bg-green-500 text-white rounded hover:bg-green-600"
                  >
                    创建
                  </button>
                  <button
                    onClick={() => setShowCreateTag(false)}
                    className="flex-1 px-2 py-1 text-sm bg-gray-300 rounded hover:bg-gray-400"
                  >
                    取消
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* Tag List */}
          <div className="space-y-1 mb-4">
            <p className="text-xs text-gray-500 px-1">可用标签：</p>
            {customTags.length === 0 ? (
              <p className="text-sm text-gray-400 px-1">暂无标签</p>
            ) : (
              customTags.map(tag => (
                <div
                  key={tag.id}
                  className="flex items-center gap-2 px-2 py-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 group"
                >
                  <span
                    className="w-3 h-3 rounded-full flex-shrink-0"
                    style={{ backgroundColor: tag.color }}
                  />
                  <span className="text-sm flex-1">{tag.name}</span>
                  <button
                    onClick={() => handleDeleteTag(tag.id)}
                    className="text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100"
                  >
                    ×
                  </button>
                </div>
              ))
            )}
          </div>

          {/* File Tags Section */}
          {selectedNodeIds.length > 0 && (
            <div className="border-t border-gray-200 dark:border-gray-700 pt-3">
              <p className="text-xs text-gray-500 px-1 mb-2">当前文件标签：</p>
              
              {/* Add Tag Menu */}
              <div className="relative mb-2">
                <button
                  onClick={() => setShowTagMenuFor(showTagMenuFor ? null : 'add')}
                  className="w-full px-2 py-1 text-sm border border-dashed rounded hover:bg-gray-50"
                >
                  + 添加标签
                </button>
                {showTagMenuFor === 'add' && customTags.length > 0 && (
                  <div className="absolute top-full left-0 right-0 mt-1 bg-white dark:bg-gray-800 border rounded shadow-lg z-10 max-h-40 overflow-y-auto">
                    {customTags.map(tag => (
                      <button
                        key={tag.id}
                        onClick={() => handleAddTagToFile(tag.id)}
                        className="w-full flex items-center gap-2 px-3 py-2 hover:bg-gray-100 dark:hover:bg-gray-700"
                      >
                        <span
                          className="w-3 h-3 rounded-full"
                          style={{ backgroundColor: tag.color }}
                        />
                        <span className="text-sm">{tag.name}</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Current File's Tags */}
              {currentFileTags.length === 0 ? (
                <p className="text-sm text-gray-400">暂无标签</p>
              ) : (
                <div className="space-y-1">
                  {currentFileTags.map(tag => (
                    <div
                      key={tag.id}
                      className="flex items-center gap-2 px-2 py-1 bg-gray-50 dark:bg-gray-700 rounded group"
                    >
                      <span
                        className="w-3 h-3 rounded-full flex-shrink-0"
                        style={{ backgroundColor: tag.color }}
                      />
                      <span className="text-sm flex-1">{tag.name}</span>
                      <button
                        onClick={() => handleRemoveTagFromFile(tag.id)}
                        className="text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};