# 小当家 (xiaodangjia) 开发文档

## 1. 项目简介

**项目名称**: xiaodangjia (小当家)
**项目类型**: Windows桌面文件管理软件
**技术栈**: Tauri 2.x + React 18 + TypeScript + SQLite

**核心理念**: 基于脑图式无尽画布的文件管理器，通过AI Agent实现文件关联分析、自动归档、智能整理。

---

## 2. 开发环境搭建

### 2.1 基础环境要求

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| Node.js | ≥18.0.0 | 前端运行时 |
| npm | ≥9.0.0 | 包管理器 |
| Rust | ≥1.70.0 | 后端运行时 |
| Cargo | 与Rust版本匹配 | Rust包管理器 |

### 2.2 安装步骤

#### Windows 环境

`powershell
# 1. 安装 Node.js
# 访问 https://nodejs.org/ 下载 LTS 版本

# 2. 安装 Rust
# 访问 https://rust-lang.org/learn/get-started
# 或运行以下命令:
winget install Rustlang.Rust.MSVC

# 3. 验证安装
node --version    # 应显示 v18.x.x 或更高
npm --version     # 应显示 9.x.x 或更高
rustc --version   # 应显示 1.70.0 或更高
cargo --version   # 应显示对应版本
`

#### 克隆项目

`powershell
# 克隆项目
git clone https://github.com/your-repo/xiaodangjia.git
cd xiaodangjia

# 安装前端依赖
npm install

# 安装额外依赖
npm install @xyflow/react zustand @monaco-editor/react @tauri-apps/api
`

### 2.3 开发命令

`powershell
# 开发模式 (前端 + 后端)
npm run tauri dev

# 仅运行前端
npm run dev

# 构建前端
npm run build

# 构建桌面应用
npm run tauri build
`

---

## 3. 项目结构

`
xiaodangjia/
├── docs/                     # 项目文档
│   ├── README.md            # 开发文档
│   └── architecture.md      # 架构设计文档
├── src/                     # React前端源码
│   ├── components/          # React组件
│   │   ├── Canvas/         # 脑图画布组件
│   │   │   ├── Canvas.tsx  # 画布主组件
│   │   │   └── FileNode.tsx # 文件节点组件
│   │   ├── Viewer/         # 文件阅读器
│   │   │   ├── FileViewer.tsx
│   │   │   └── ViewerPanel.tsx
│   │   ├── Sidebar/        # 侧边栏
│   │   │   └── Sidebar.tsx
│   │   └── common/         # 通用组件
│   │       └── TabBar.tsx
│   ├── stores/             # Zustand状态管理
│   │   └── appStore.ts
│   ├── services/           # Tauri API调用
│   │   └── api.ts
│   ├── types/              # TypeScript类型定义
│   │   └── index.ts
│   ├── hooks/              # 自定义Hooks
│   ├── App.tsx             # 应用入口
│   └── main.tsx            # React入口
├── src-tauri/              # Rust后端源码
│   ├── src/
│   │   ├── lib.rs          # 主逻辑和Tauri命令
│   │   └── main.rs         # 程序入口
│   ├── Cargo.toml          # Rust依赖配置
│   ├── tauri.conf.json     # Tauri配置
│   └── capabilities/       # 权限配置
│       └── default.json
├── package.json            # npm配置
├── vite.config.ts          # Vite配置
└── tsconfig.json           # TypeScript配置
`

---

## 4. 技术架构

### 4.1 前端架构

`
┌─────────────────────────────────────────────┐
│              React 18 应用                   │
├─────────────────────────────────────────────┤
│  组件层 (Components)                         │
│  ├── Canvas (React Flow 脑图画布)            │
│  ├── Sidebar (项目管理)                      │
│  ├── Viewer (多格式文件阅读器)               │
│  └── TabBar (多标签页管理)                   │
├─────────────────────────────────────────────┤
│  状态层 (Zustand)                            │
│  ├── 项目状态 (projects, currentProject)     │
│  ├── 文件节点 (fileNodes, relations)         │
│  ├── 标签页 (openTabs, activeTabId)         │
│  └── 画布状态 (viewport, selectedNodeIds)   │
├─────────────────────────────────────────────┤
│  服务层 (Tauri IPC)                          │
│  └── 通过 @tauri-apps/api 调用Rust后端       │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│              Tauri IPC Bridge                │
└─────────────────────────────────────────────┘
`

### 4.2 后端架构

`
┌─────────────────────────────────────────────┐
│              Rust Tauri 后端                 │
├─────────────────────────────────────────────┤
│  命令层 (Tauri Commands)                     │
│  ├── 项目管理 (create_project, list_projects)│
│  ├── 文件操作 (scan_directory, read_file)   │
│  ├── 节点管理 (update_node_position)         │
│  └── AI分析 (analyze_relations, generate_tags)│
├─────────────────────────────────────────────┤
│  数据层                                      │
│  ├── SQLite (结构化数据: 项目、节点、关联)   │
│  └── 文件系统 (walkdir, fs)                  │
└─────────────────────────────────────────────┘
`

---

## 5. API 参考

### 5.1 前端 API (services/api.ts)

| 函数 | 说明 | 参数 |
|------|------|------|
| createProject(name, rootPath) | 创建项目 | 
ame: string, ootPath: string |
| listProjects() | 获取项目列表 | - |
| scanDirectory(projectId, path) | 扫描目录 | projectId: string, path: string |
| eadFileContent(path) | 读取文件内容 | path: string |
| writeFileContent(path, content) | 写入文件 | path: string, content: string |
| updateNodePosition(id, x, y) | 更新节点位置 | id: string, x: number, y: number |
| nalyzeFileRelations(projectId) | 分析文件关联 | projectId: string |
| generateTags(projectId, fileId) | 生成文件标签 | projectId: string, ileId: string |
| searchFiles(projectId, query) | 搜索文件 | projectId: string, query: string |

### 5.2 Rust 命令 (lib.rs)

`ust
// 项目管理
#[tauri::command] fn create_project(name: String, root_path: String) -> Project
#[tauri::command] fn list_projects() -> Vec<Project>
#[tauri::command] fn get_project(id: String) -> Project
#[tauri::command] fn delete_project(id: String) -> Result<()>

// 文件操作
#[tauri::command] fn scan_directory(project_id: String, path: String) -> Vec<FileNode>
#[tauri::command] fn read_file_content(path: String) -> FileContent
#[tauri::command] fn write_file_content(path: String, content: String) -> Result<()>
#[tauri::command] fn delete_file(path: String) -> Result<()>
#[tauri::command] fn rename_file(old_path: String, new_path: String) -> Result<()>

// 节点管理
#[tauri::command] fn update_node_position(id: String, x: f64, y: f64) -> Result<()>

// AI分析
#[tauri::command] fn analyze_file_relations(project_id: String) -> Vec<FileRelation>
#[tauri::command] fn generate_tags(project_id: String, file_id: String) -> Vec<String>
#[tauri::command] fn search_files(project_id: String, query: String) -> Vec<FileNode>
#[tauri::command] fn find_similar_files(project_id: String, file_id: String) -> Vec<FileNode>

// 系统交互
#[tauri::command] fn open_directory_dialog() -> Option<String>
`

---

## 6. 支持的文件格式

### 6.1 代码/文本 (Monaco Editor)

**前端**: js, jsx, 	s, 	sx, html, css, scss, less

**后端**: py, s, go, java, c, cpp, h, hpp, php, b, swift, kt, scala

**脚本**: sh, ash, ps1, at, cmd

**配置**: json, xml, yaml, yml, 	oml, ini, sql

**文档**: md, 	xt, lua, , pl

### 6.2 图片

png, jpg, jpeg, gif, mp, webp, svg, ico, 	iff, psd, i, eps

### 6.3 视频

mp4, webm, mkv, vi, mov, wmv, lv, 4v, m4v, mpg, mpeg, 3gp, ogv, 	s, ob, m, mvb

### 6.4 音频

mp3, wav, ogg, lac, ac, m4a, wma, opus, pe, lac, mid, midi, iff, c3

### 6.5 Office文档

doc, docx, xls, xlsx, ppt, pptx, odt, ods, odp, tf, csv

### 6.6 电子书

epub, mobi, zw3, djvu, djv, chm, cbr, cbz, b2, lit

### 6.7 压缩包

zip, ar, 7z, 	ar, gz, z2, xz, iso, dmg, deb, pm

---

## 7. 数据库设计

### 7.1 SQLite 表结构

`sql
-- 项目表
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at TEXT,
    updated_at TEXT
);

-- 文件节点表
CREATE TABLE file_nodes (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    path TEXT,
    name TEXT,
    extension TEXT,
    size INTEGER,
    created_at TEXT,
    modified_at TEXT,
    tags TEXT,
    parent_id TEXT,
    position_x REAL,
    position_y REAL,
    is_collapsed INTEGER,
    is_directory INTEGER
);

-- 文件关联表
CREATE TABLE file_relations (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    source_id TEXT,
    target_id TEXT,
    relation_type TEXT,
    confidence REAL
);

-- Agent日志表
CREATE TABLE agent_logs (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    action TEXT,
    result TEXT,
    created_at TEXT
);
`

---

## 8. 配置说明

### 8.1 tauri.conf.json

`json
{
  \"productName\": \"小当家\",
  \"version\": \"0.1.0\",
  \"identifier\": \"com.xiaodangjia.app\",
  \"app\": {
    \"windows\": [{
      \"title\": \"小当家 - 脑图式文件管理器\",
      \"width\": 1400,
      \"height\": 900,
      \"minWidth\": 800,
      \"minHeight\": 600,
      \"center\": true,
      \"resizable\": true
    }]
  }
}
`

### 8.2 Cargo.toml 关键依赖

`	oml
[dependencies]
tauri = { version = \"2\", features = [\"tray-icon\"] }
rusqlite = { version = \"0.32\", features = [\"bundled\"] }
walkdir = \"2\"
notify = \"7\"
uuid = { version = \"1\", features = [\"v4\", \"serde\"] }
chrono = { version = \"0.4\", features = [\"serde\"] }
tokio = { version = \"1\", features = [\"full\"] }
`

---

## 9. 常见问题

### 9.1 编译错误

**问题**: error: failed to run custom build command for 'tauri'

**解决**: 确保已安装Rust并配置好环境变量
`powershell
# 重新加载环境
refreshenv
rustc --version
`

### 9.2 WebView2 缺失

**问题**: 运行时提示找不到 WebView2

**解决**: Windows 10/11 已内置WebView2，如遇问题可手动下载安装：
https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### 9.3 权限问题

**问题**: 无法访问文件系统

**解决**: 检查 capabilities/default.json 中的fs权限配置

---

## 10. 开发规范

### 10.1 命名规范

- 组件文件: PascalCase (FileViewer.tsx)
- 工具函数: camelCase (eadFileContent)
- 常量: UPPER_SNAKE_CASE
- CSS类: kebab-case

### 10.2 提交规范

`
feat: 添加文件关联分析功能
fix: 修复节点拖拽位置保存问题
docs: 更新开发文档
refactor: 优化文件扫描性能
`

---

## 11. 后续规划

- [ ] 向量数据库集成 (Chroma/Milvus)
- [ ] AI Agent 智能分析
- [ ] 文件版本管理
- [ ] 插件系统
- [ ] 云同步功能

---

*最后更新: 2026-06-10*
