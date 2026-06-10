import { useAppStore } from '../../stores/appStore';

export default function TabBar() {
  const { openTabs, activeTabId, setActiveTab, closeTab } = useAppStore();

  if (openTabs.length === 0) {
    return null;
  }

  const getFileIcon = (name: string) => {
    const ext = name.split('.').pop() || '';
    const iconMap: Record<string, string> = {
      js: '📜', jsx: '⚛️', ts: '📘', tsx: '⚛️', py: '🐍', 
      md: '📝', json: '📋', html: '🌐', css: '🎨',
    };
    return iconMap[ext] || '📄';
  };

  return (
    <div className="flex items-center bg-gray-100 border-b border-gray-200 overflow-x-auto">
      {openTabs.map((tab) => (
        <div
          key={tab.id}
          className={`
            flex items-center gap-2 px-3 py-2 cursor-pointer border-r border-gray-200 min-w-[120px] max-w-[200px]
            ${activeTabId === tab.id 
              ? 'bg-white border-b-2 border-b-blue-500' 
              : 'bg-gray-50 hover:bg-gray-100'
            }
          `}
          onClick={() => setActiveTab(tab.id)}
        >
          <span>{getFileIcon(tab.name)}</span>
          <span className="flex-1 truncate text-sm">{tab.name}</span>
          {tab.isModified && <span className="text-orange-500">●</span>}
          <button
            onClick={(e) => {
              e.stopPropagation();
              closeTab(tab.id);
            }}
            className="text-gray-400 hover:text-gray-600 text-xs"
          >
            ✕
          </button>
        </div>
      ))}
    </div>
  );
}
