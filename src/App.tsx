import { useEffect, useState } from 'react';
import { useAppStore } from './stores/appStore';
import DarkModeToggle from './components/common/DarkModeToggle';
import { Sidebar } from './components/Sidebar';
import { Canvas } from './components/Canvas';
import { TabBar } from './components/common';
import { ViewerPanel } from './components/Viewer';
import './App.css';

function App() {
  const { openTabs } = useAppStore();
  const hasOpenTabs = openTabs.length > 0;
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    console.log('App mounted');
    setMounted(true);
  }, []);

  return (
    <div className="h-screen w-screen flex overflow-hidden bg-red-500">
      {/* Very visible debug - this should show if React is working */}
      <div style={{
        position: 'fixed',
        top: 0,
        left: 0,
        background: 'red',
        color: 'white',
        padding: '20px',
        zIndex: 99999,
        fontSize: '24px'
      }}>
        REACT MOUNTED: {mounted ? 'YES' : 'NO'}
      </div>

      {/* Sidebar */}
      <Sidebar />

      {/* Dark Mode Toggle */}
      <DarkModeToggle />

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
