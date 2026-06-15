import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// 添加错误捕获
window.onerror = (message, source, lineno, colno, error) => {
  console.error('Global error:', message, source, lineno, colno, error);
};

window.onunhandledrejection = (event) => {
  console.error('Unhandled promise rejection:', event.reason);
};

// 测试 React 是否能渲染
console.log('Starting React app...');

const rootElement = document.getElementById("root");
console.log('Root element:', rootElement);

ReactDOM.createRoot(rootElement as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

console.log('React app rendered');