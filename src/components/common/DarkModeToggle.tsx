import { useAppStore } from '../../stores/appStore';

export default function DarkModeToggle() {
  const { darkMode, setDarkMode } = useAppStore();

  return (
    <button
      onClick={() => setDarkMode(!darkMode)}
      className="fixed top-3 right-3 z-50 w-8 h-8 flex items-center justify-center rounded-full bg-white/80 backdrop-blur-sm shadow-md border border-gray-200 hover:bg-gray-100 transition-all text-sm"
      title={darkMode ? '切换到明亮模式' : '切换到深色模式'}
    >
      {darkMode ? '☀️' : '🌙'}
    </button>
  );
}
