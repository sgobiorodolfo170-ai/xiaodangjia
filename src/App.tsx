import { useAppStore } from './stores/appStore';
import { Sidebar } from './components/Sidebar';
import { Canvas } from './components/Canvas';
import { TabBar } from './components/common';
import { ViewerPanel } from './components/Viewer';
import './App.css';

function App() {
  const { openTabs } = useAppStore();
  const hasOpenTabs = openTabs.length > 0;

  return (
    <div className="h-screen w-screen flex overflow-hidden">
      {/* Sidebar */}
      <Sidebar />

      {/* Main Content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Canvas Area */}
        <div className={`flex-1 ${hasOpenTabs ? 'h-1/2' : 'h-full'}`}>
          <Canvas />
        </div>

        {/* Viewer Panel (split view) */}
        {hasOpenTabs && (
          <div className="h-1/2 border-t border-gray-300">
            <TabBar />
            <div className="h-[calc(100%-36px)]">
              <ViewerPanel />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
