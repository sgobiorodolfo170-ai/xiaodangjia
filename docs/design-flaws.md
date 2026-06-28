---
title: 功能性设计缺陷分析与修复计划
description: 小当家项目全量设计缺陷清单，按严重性分级，含修复方案与路线图
date: 2026-06-17
---

# 功能性设计缺陷分析与修复计划

> **状态**: 待评审
> **分析方法**: 三个并行代码审查代理（Rust 后端 / 前端架构 / 配置测试）通读全量源码
> **覆盖范围**: `src-tauri/src/*`、`src/**/*`、配置文件、测试

## 概述

本轮审查共识别 **37 个功能性设计缺陷**，按严重性分级如下：

| 严重性 | 数量 | 特征 | 代表问题 |
|--------|------|------|----------|
| 🔴 P0 | 8 | 数据丢失 / 安全 / 崩溃 | rescan 覆盖布局、未 await 删除、路径校验缺失、非原子删插 |
| 🟠 P1 | 14 | 数据不一致 / 体验差 | 双标签不同步、编辑内容丢失、Canvas 全量重建、Mutex 阻塞 |
| 🟡 P2 | 10 | 架构问题 / 可维护性 | delete/rename 不更新 DB、plugin 空壳、类型断言 |
| ⚪ P3 | 5 | 性能小优化 | 正则未缓存、全量拉取 |

---

## 🔴 P0 严重缺陷

> 直接导致数据丢失、安全漏洞或应用崩溃，应最优先修复。

### P0-1 Watcher rescan 覆盖画布布局，丢失用户所有手动调整

- **位置**: `src/components/Sidebar/Sidebar.tsx:65-66`
- **问题**: 文件系统变化触发 watcher 事件后，`debouncedRescan` 调用 `scanDirectory` 并用 `setFileNodes(nodes)` **全量替换** `fileNodes`。新扫描的节点使用默认布局位置。
- **影响**: 
  - 所有 `updateNodePosition` 修改的拖动布局丢失
  - 所有 `updateFileTags` 标签丢失
  - 所有 `isCollapsed` 折叠状态丢失
  - 用户精心排列的脑图随时可能被一次外部保存操作重置
- **修复方案**: 
  1. rescan 时改用**增量合并**而非全量替换：以路径为 key，保留旧节点的 `positionX/positionY/isCollapsed/tags`，只更新 `size/modifiedAt` 等文件系统属性
  2. 新增节点使用自动布局算法放置在空闲区域
  3. 已删除的节点才从画布移除
- **工作量**: M

### P0-2 `deleteFilePermanent` 未 await，UI 显示已删除但文件仍在磁盘

- **位置**: `src/components/Canvas/FileNode.tsx:117`
- **问题**: 对比 `handleTrash`（第 104 行有 `await`），`api.deleteFilePermanent(nodeData.path)` **没有 await**。`try/catch` 包裹的是同步返回的 Promise 而非实际结果，删除失败时 `removeFileNode(nodeData.id)` 仍然执行。
- **影响**: 严重数据不一致——用户以为删除成功，但文件实际仍在磁盘上，且画布已移除节点。
- **修复方案**: 
  ```tsx
  const handleDeletePermanent = async () => {
    if (!confirm(...)) return;
    try {
      await api.deleteFilePermanent(nodeData.path);  // 加 await
      removeFileNode(nodeData.id);
      alert('已永久删除');
    } catch (e) {
      alert('删除失败: ' + (e instanceof Error ? e.message : String(e)));
    }
    setContextMenu(null);
  };
  ```
- **工作量**: S

### P0-3 Watcher 后端线程无法停止，切换项目累积泄漏

- **位置**: 
  - 后端 `src-tauri/src/watcher.rs:20-71`（`start_file_watcher`）
  - 前端 `src/components/Sidebar/Sidebar.tsx:97-98`（cleanup 仅 `unlisten()`）
- **问题**: 
  1. 前端 cleanup 只取消事件监听，**不停止后端 watcher 线程**
  2. `watcher` 变量在 `watcher.rs:36` 后未被移动或存储，在第一次循环迭代结束即被 drop
  3. 之后 `rx.recv_timeout`（`watcher.rs:64`）永远超时空转，线程无法退出
  4. 每次切换项目调用 `startFileWatcher` 创建新线程，旧线程不回收
- **影响**: 内存泄漏 + 重复 rescan 事件触发 + CPU 空转浪费
- **修复方案**: 
  1. 在 `AppState` 增加 `watcher_handles: Mutex<HashMap<String, JoinHandle>>` 存储按 projectId 索引的线程句柄
  2. 新增 `stop_file_watcher(project_id)` Tauri command，通过 channel 发送退出信号并 join 线程
  3. 将 `RecommendedWatcher` 移入循环内持续持有，或存入 AppState
  4. 前端 cleanup 中调用 `stopFileWatcher` 再 `unlisten`
- **工作量**: M

### P0-4 切换项目不清理 tabs/selection，ViewerPanel 显示错误项目文件

- **位置**: `src/components/Sidebar/Sidebar.tsx:105-118`（`handleSelectProject`）
- **问题**: 
  1. 只更新 `currentProject` 和 `fileNodes`，不清理 `openTabs`/`activeTabId`/`selectedNodeIds`/`relations`
  2. `setCurrentProject(project)`（第 106 行）在 `scanDirectory`（第 109 行）**之前**调用
  3. 若扫描失败，UI 显示新项目名称但 `fileNodes` 仍是旧项目数据
- **影响**: 
  - 切换项目后 ViewerPanel 仍显示旧项目文件内容
  - 点击旧 tab 时路径不匹配导致错误
  - 扫描失败时数据错乱
- **修复方案**: 
  ```tsx
  const handleSelectProject = async (project) => {
    setIsLoading(true);
    try {
      const nodes = await api.scanDirectory(project.id, project.rootPath);
      // 扫描成功后才切换状态
      clearAllTabs();          // 新增 store action
      setSelectedNodeIds([]);
      setRelations([]);
      setSearchResults([]);
      setFileNodes(nodes);
      setCurrentProject(project);  // 移到最后
    } catch (error) {
      toast.error('加载项目失败');
    } finally {
      setIsLoading(false);
    }
  };
  ```
- **工作量**: S

### P0-5 路径校验缺失——可在项目目录外任意操作文件

- **位置**: 
  - `src-tauri/src/lib.rs:234-279`（`read_file_content` / `write_file_content`）
  - `src-tauri/src/lib.rs:606-658`（`create_directory` / `move_file` / `copy_file`）
  - `src-tauri/src/lib.rs:1113-1124`（`delete_file_permanent`）
  - `src-tauri/src/lib.rs:747-787`（`batch_operation` 内部函数）
- **问题**: 这些命令接受任意路径参数，没有调用 `validate_path_in_project`。前端可以传入 `C:\Windows\System32\...` 或 `/etc/passwd`。
- **影响**: 
  - 可读取任意敏感文件（如密钥、配置）
  - 可写入任意系统文件
  - `delete_file_permanent` 可删除系统关键文件
- **修复方案**: 
  1. 所有文件操作命令入口处调用 `validate_path_in_project(path, project_id, state)?`
  2. 修复 `validate_path_in_project` 对不存在路径（如 rename 的 new_path）的处理——先用父目录 canonicalize
  3. 防止符号链接逃逸：canonicalize 后再校验是否在项目根下
- **工作量**: M

### P0-6 `scan_directory` 非原子"先删后插"，中断即数据全丢

- **位置**: `src-tauri/src/lib.rs:208-212`
- **问题**: 
  ```rust
  db.delete_file_nodes_by_project(&project_id)?;
  for node in &nodes {
      let _ = db.insert_file_node(node);  // 错误被吞掉
  }
  ```
  删除旧节点和插入新节点不在同一事务中。插入错误被 `let _ =` 静默忽略。配合 `ON DELETE CASCADE`，`file_relations`/`favorites`/`file_tags`/`file_edit_history` 级联丢失。
- **影响**: 扫描中断后，项目所有文件节点数据丢失，且关联的收藏、标签、编辑历史全部丢失。
- **修复方案**: 
  1. 使用已有的 `insert_file_nodes_batch`（`db.rs`，已实现事务包装）
  2. 将"删除 + 批量插入"包进单一事务：`delete_and_replace_nodes(project_id, nodes)`
  3. 移除所有 `let _ = db.*`，改为 `?` 传播或记录日志
- **工作量**: S

### P0-7 `trash_file` PowerShell 命令注入风险

- **位置**: `src-tauri/src/lib.rs:679-710`、`834-863`
- **问题**: 
  ```rust
  let script = format!(
      r#"...DeleteFile('{}', ...)"#,
      path.replace("'", "''")
  );
  Command::new("powershell").args(["-NoProfile", "-Command", &script]).output()
  ```
  仅做单引号转义，PowerShell 字符串转义规则更复杂，路径含 `)`、`"`、反引号等可能逃逸。
- **影响**: 恶意构造的文件路径可能执行任意 PowerShell 命令。
- **修复方案**: 
  - **推荐**: 引入 [`trash`](https://crates.io/crates/trash) crate，跨平台、无 shell 调用
  - **备选**: 用 `-File` 参数传递临时脚本文件，或 Base64 编码命令
- **工作量**: S

### P0-8 DB Mutex 长时间持有，阻塞全部命令导致 UI 冻结

- **位置**: 
  - `src-tauri/src/lib.rs:208-212`（`scan_directory`，锁内逐行 INSERT）
  - `src-tauri/src/lib.rs:432-464`（`find_similar_by_content`，锁内读 500 文件）
  - `src-tauri/src/lib.rs:468-502`（`find_similar_by_embedding`，锁内读 500 文件）
  - `src-tauri/src/lib.rs:538-571`（`analyze_import_relations`，锁内 O(N²) AST）
- **问题**: 所有 Tauri command 共享 `state.db` 的单一 Mutex。这些函数获取锁后，在锁的保护下做大量文件 I/O 和 CPU 密集计算，期间所有其他 command 被阻塞。已有 `insert_file_nodes_batch` 却未在 scan 中使用。
- **影响**: 一次扫描或相似度搜索冻结整个应用数秒至数十秒。
- **修复方案**: 
  1. `scan_directory` 改为 `async fn`，用 `tauri::async_runtime::spawn_blocking` 将文件遍历放后台线程，只在最后获取锁做批量 DB 写入
  2. 文件遍历阶段先完成（不持锁），仅在 `insert_file_nodes_batch` 时短暂持锁
  3. 相似度搜索将文件内容读取移到锁外（先 `get_file_nodes` 拿到路径列表，释放锁，再并行读取文件）
- **工作量**: M

---

## 🟠 P1 重要缺陷

> 数据不一致或明显体验问题，影响核心使用流程。

### P1-1 双重标签系统不同步

- **位置**: `src/stores/appStore.ts:80-84`（`updateFileTags`）vs `107-122`（`fileCustomTags` 系列）
- **问题**: `FileNode.tags`（内嵌 string[]）和 `fileCustomTags`（独立 Record）是两套独立数据，无同步逻辑。`FavoritesTagsPanel` 修改标签只更新 `fileCustomTags`，不影响 `FileNode.tags`。
- **影响**: Canvas 上的标签显示与 FavoritesTagsPanel 显示不一致。
- **修复方案**: 统一为单一数据源（推荐用 `fileCustomTags`，`FileNode.tags` 改为派生 selector），或两者同步更新。
- **工作量**: M

### P1-2 全局单一 `isLoading` 无法区分并发操作

- **位置**: `src/stores/appStore.ts:53-54`
- **问题**: 扫描目录、AI 分析、项目创建共用同一布尔值。
- **影响**: AI 分析进行中切换项目，`finally` 会提前关闭加载指示器。
- **修复方案**: 改为计数器 `loadingCount` 或按操作类型分键（`isScanning`/`isAnalyzing`）。
- **工作量**: S

### P1-3 `favoriteIds` 乐观更新不清理，删除文件后出现幽灵收藏

- **位置**: `src/stores/appStore.ts:87-94`
- **问题**: `addFavoriteId`/`removeFavoriteId` 乐观更新，但文件删除后不从 `favoriteIds` 移除。
- **修复方案**: `removeFileNode` 时级联清理 `favoriteIds`；或后端删除时返回受影响 favorites。
- **工作量**: S

### P1-4 ViewerPanel 编辑内容仅在组件本地 state

- **位置**: `src/components/Viewer/ViewerPanel.tsx:13`（`const [content, setContent] = useState('')`）
- **问题**: 编辑中内容存组件本地 state，切换 tab 卸载即丢失。
- **影响**: 用户切换 tab 后未保存内容丢失。
- **修复方案**: 将 `content`/`isModified` 提升到 `OpenTab` 结构存入 store，或用 `useRef` + 缓存 Map。
- **工作量**: M

### P1-5 关闭未保存 tab 无确认提示

- **位置**: `src/stores/appStore.ts:140-148`（`closeTab`）
- **问题**: 不检查 `isModified`，直接关闭。
- **修复方案**: `closeTab` 检查 `isModified`，返回需确认标志；前端拦截并弹确认框。
- **工作量**: S

### P1-6 历史恢复后标记 modified 但不自动保存

- **位置**: `src/components/Viewer/ViewerPanel.tsx:91-103`（`handleRestore`）
- **问题**: `updateTab(activeTab!.id, { isModified: true })`，但内容只在本地 state，不保存就切 tab 即丢失。还用了 `activeTab!` 非空断言。
- **修复方案**: 恢复后立即 `handleSave()`，或至少保证 content 持久化到 store；移除非空断言加守卫。
- **工作量**: S

### P1-7 无自动保存、无 Ctrl+S 快捷键

- **位置**: `src/components/Viewer/ViewerPanel.tsx` 全文
- **问题**: 编辑器面板无自动保存，无键盘快捷键注册。
- **修复方案**: 
  - debounce 1s 自动保存
  - `useEffect` 注册 `keydown`，`Ctrl+S`/`Cmd+S` 触发 `handleSave` 并 `preventDefault`
- **工作量**: S

### P1-8 并发编辑无冲突检测

- **位置**: `src/components/Viewer/ViewerPanel.tsx:55-67`
- **问题**: watcher 触发外部修改后，ViewerPanel 不检测文件变化（`useEffect` 只监听 path 变化）。保存会静默覆盖。
- **修复方案**: 保存前比较文件 mtime/hash，不一致则提示冲突；或用文件锁。
- **工作量**: M

### P1-9 Canvas 全量重建 nodes/edges，拖动后画布闪跳

- **位置**: `src/components/Canvas/Canvas.tsx:75-111`
- **问题**: `initialNodes`/`initialEdges` 在组件体计算（未 memo），通过 `useEffect` 全量 `setNodes`/`setEdges`。拖动节点触发 `updateNodePosition` → store 变化 → 全量替换。
- **影响**: ReactFlow 选中状态/动画丢失，视觉闪跳。
- **修复方案**: 
  1. 用 `useMemo` 包裹转换
  2. 改为**增量更新**：`fileNodes` 变化时 diff 出增删改，调用 `addNodes`/`removeNodes`/`updateNodeData`
  3. 不要在每次 `fileNodes` 变化时 `setNodes(initialNodes)`
- **工作量**: L

### P1-10 `parent_id` 查找 O(N²)

- **位置**: `src-tauri/src/lib.rs:136-143`
- **问题**: 每个节点线性搜索 `nodes.iter().find(|n| n.path == parent_str)`。
- **影响**: 10000 文件需 ~5000 万次字符串比较。
- **修复方案**: 遍历前构建 `HashMap<String /*path*/, String /*id*/>`，查找 O(1)。
- **工作量**: S

### P1-11 相似度搜索无缓存，每次重建索引

- **位置**: `src-tauri/src/lib.rs:432-464`、`468-502`
- **问题**: 每次 API 调用都读 500 文件、构建 TF-IDF/embedding。
- **修复方案**: 索引持久化到 DB（`file_embeddings` 表），文件变更时增量更新；或内存 LRU 缓存。
- **工作量**: L

### P1-12 FileTree 无虚拟化，大项目渲染卡顿

- **位置**: `src/components/Sidebar/FileTree.tsx:93-132`
- **问题**: `renderNode` 递归渲染整棵树到 DOM，无虚拟列表。
- **修复方案**: 引入 `react-window` 或 `@tanstack/react-virtual` 做扁平化虚拟渲染。
- **工作量**: M

### P1-13 所有错误反馈用 `alert()`，阻塞式

- **位置**: `FileNode.tsx:79,83,96,107,109,119,121`、`BatchToolbar.tsx` 多处、`Sidebar.tsx:251,253`
- **问题**: `alert()` 阻塞页面，批量删除失败时中断流程；Tauri 下可能不显示。
- **修复方案**: 引入 toast 库（如 `react-hot-toast`），统一错误展示组件。
- **工作量**: S

### P1-14 API 层无 try-catch，Rust panic 直接暴露

- **位置**: `src/services/api.ts:1-263` 全文件
- **问题**: 所有 `invoke` 直接 return，错误格式不统一。
- **修复方案**: 封装统一 `safeInvoke` 包装器，转换错误为友好消息 + 错误码。
- **工作量**: M

---

## 🟡 P2 中等缺陷

### P2-1 `delete_file` / `rename_file` 不更新数据库

- **位置**: `src-tauri/src/lib.rs:316-331`、`334-344`
- **问题**: 磁盘操作后 `file_nodes` 表仍是旧值，必须重新 scan 才同步。
- **修复方案**: 操作成功后同步调用 `db.delete_file_node(id)` 或 `db.update_file_node_path(id, new_path, new_name)`。
- **工作量**: S

### P2-2 `import_project` favorites 用旧 file_id，外键悬空

- **位置**: `src-tauri/src/lib.rs:983-985`
- **问题**: 已有 `node_id_map`（第 951 行）重映射 ID，但 favorites 没用它。
- **修复方案**: `for file_id in data.favorites { if let Some(new_id) = node_id_map.get(&file_id) { db.add_favorite(&project.id, new_id)?; } }`。
- **工作量**: S

### P2-3 `export_project` 导出全局 tags 而非项目 tags

- **位置**: `src-tauri/src/lib.rs:910-911` 调用 `db.list_tags()`
- **问题**: 导出包含所有项目标签，导入时重复创建。
- **修复方案**: 新增 `list_tags_by_project(project_id)`，按项目过滤。
- **工作量**: S

### P2-4 plugin_system 是空壳死代码

- **位置**: `src-tauri/src/plugin_system.rs` 全文、`lib.rs:575-580`
- **问题**: `Plugin` trait 无业务接口，`list_plugins` 每次新建即丢弃。
- **修复方案**: 要么删除整个模块，要么定义真实业务接口（如 `analyze(node) -> Insight`）并持久化 registry。
- **工作量**: M（删除）/ L（重写）

### P2-5 四个相似度系统功能重叠，命名误导

- **位置**: 
  - `watcher.rs:205-218`（扩展名+字符串相似度）
  - `tfidf.rs:125-141`（TF-IDF）
  - `semantic_embedding.rs:174-187`（规则匹配伪装成 embedding）
  - `semantic_search.rs:94-156`（同义词搜索）
- **问题**: 无统一接口，结果格式不一，`semantic_embedding` 命名误导（非真实向量嵌入）。
- **修复方案**: 定义 `trait FileSimilarity { fn find_similar(&self, target, candidates) -> Vec<ScoredNode>; }`，统一结果类型；或合并为单一可配置引擎。
- **工作量**: L

### P2-6 `watcher.rs` 混合 5 种不相关职责

- **位置**: `src-tauri/src/watcher.rs` 全文
- **问题**: 文件监听 + 关系分析 + 标签生成 + 搜索 + 相似度查找，全塞一个文件。
- **修复方案**: 拆分为 `watcher.rs`（仅监听）、`analysis/`（关系/标签/相似度）、`search.rs`。
- **工作量**: M

### P2-7 `ProjectExport.relations` 使用 `any[]`

- **位置**: `src/services/api.ts:226`
- **修复方案**: 改为 `FileRelation[]`。
- **工作量**: S

### P2-8 `SearchFilterPanel.onResults` 回调参数 `any[]`

- **位置**: `src/components/Sidebar/SearchFilterPanel.tsx:6`
- **修复方案**: 改为 `FileNode[]`。
- **工作量**: S

### P2-9 FileNodeComponent 双重类型断言

- **位置**: `src/components/Canvas/FileNode.tsx:36`（`data as unknown as FileNodeType`）
- **修复方案**: 定义 ReactFlow `Node<FileNodeType>` 泛型，消除断言。
- **工作量**: S

### P2-10 `catch (e)` 隐式 any，`alert('失败:'+e)` 显示 `[object Object]`

- **位置**: `FileNode.tsx:83,97,109,120` 等多处
- **修复方案**: `catch (e: unknown)` + `(e instanceof Error ? e.message : String(e))`，或抽取 `formatError(e)` 工具函数。
- **工作量**: S

---

## ⚪ P3 低优先级

### P3-1 `ast_parser.rs` 每次重新编译正则
- **位置**: `ast_parser.rs:14,26,37,49,62,75,82,91`
- **方案**: `std::sync::OnceLock<Regex>` 或 `lazy_static!`。**工作量**: S

### P3-2 `is_favorite` 全量拉取再 contains
- **位置**: `src-tauri/src/lib.rs:1147-1151`
- **方案**: `SELECT EXISTS(SELECT 1 FROM favorites WHERE file_id = ?)`。**工作量**: S

### P3-3 `tfidf.rs` `tokens.contains` 线性搜索
- **位置**: `src-tauri/src/tfidf.rs:62-64`
- **方案**: `tokens` 改用 `HashSet<String>`。**工作量**: S

### P3-4 `db.rs` 多处 `filter_map(|r| r.ok())` 吞错误
- **位置**: `db.rs:174,241,266,307,347,492,561`
- **方案**: 改为 `?` 传播或记录 warn 日志。**工作量**: S

### P3-5 `StatsPanel` `formatSize` 在组件内定义
- **位置**: `src/components/Sidebar/StatsPanel.tsx:62-67`
- **方案**: 提取为模块级函数。**工作量**: S

---

## 配置与测试维度

### 测试覆盖极低
- **现状**: 仅 `src/stores/appStore.test.ts` 测 Zustand store（12 个用例）
- **缺失**: 
  - 后端零测试——`scan_directory`、`write_file_content`、编辑历史、路径校验、相似度搜索全部无覆盖
  - 前端组件无测试——Canvas/FileNode/ViewerPanel/Sidebar 均无
- **建议**: 
  - 后端用 `#[cfg(test)]` 模块 + 临时目录测 DB 操作和路径校验
  - 前端用 `@testing-library/react` 测关键交互（切换项目、编辑保存、删除）
- **优先级**: 配合 P0 修复同步补充测试

### fs scope 过宽
- **位置**: `src-tauri/capabilities/default.json:19-24`（`{ "path": "**" }`）
- **问题**: 允许访问全盘，配合 P0-5 路径校验缺失放大风险。
- **建议**: scope 限制为 `$APPDATA/*` 和用户主动选择的项目目录。

### 无数据库迁移机制
- **现状**: `init_tables` 直接 `CREATE TABLE IF NOT EXISTS`，无 schema 版本管理。
- **风险**: 未来字段变更会破坏旧数据库。
- **建议**: 增加 `schema_version` 表 + 顺序迁移脚本。

### CSP 配置为 null
- **位置**: `src-tauri/tauri.conf.json`
- **建议**: 生产环境配置合理 CSP 限制脚本/样式来源。

### 路径硬编码 Windows 分隔符
- **位置**: 多处 `.replace("\\", "/")`
- **建议**: 统一用 `std::path::Path` 处理，或封装跨平台路径工具。

---

## 建议修复路线图

### 阶段一：见效快（解决当前反馈的两个问题）
> 预估 1-2 天，直接改善"加载无响应"和"界面跳转"

| 编号 | 任务 | 文件 |
|------|------|------|
| P0-6 | scan_directory 改用 `insert_file_nodes_batch` + 事务 | `lib.rs`、`db.rs` |
| P0-8 | scan_directory 改 async，文件遍历移出锁 | `lib.rs` |
| 新增 | WalkDir 过滤 `node_modules`/`.git`/`target`/`dist`/`__pycache__`/`.venv`/`build` | `lib.rs` |
| P1-10 | parent_id 查找改 HashMap | `lib.rs` |
| 新增 | Canvas 加载动画 + fitView 平滑过渡 | `Canvas.tsx` |

### 阶段二：数据安全
> 预估 2-3 天，消除数据丢失和安全风险

| 编号 | 任务 | 文件 |
|------|------|------|
| P0-1 | rescan 改增量合并，保留用户布局 | `Sidebar.tsx`、`lib.rs` |
| P0-2 | deleteFilePermanent 加 await | `FileNode.tsx` |
| P0-3 | watcher 资源管理（stop 命令 + AppState 存储） | `watcher.rs`、`lib.rs`、`Sidebar.tsx` |
| P0-4 | 切换项目清理状态 + 扫描成功后才切换 | `Sidebar.tsx`、`appStore.ts` |
| P0-5 | 所有文件操作加路径校验 | `lib.rs` |
| P0-7 | trash_file 换 `trash` crate | `lib.rs`、`Cargo.toml` |

### 阶段三：体验完善
> 预估 2-3 天

| 编号 | 任务 |
|------|------|
| P1-4 | 编辑内容持久化到 store |
| P1-5/6/7 | 未保存确认 + 恢复后自动保存 + Ctrl+S + 自动保存 |
| P1-1 | 双标签系统统一 |
| P1-13 | alert 改 toast |
| P1-14 | API 层统一错误处理 |
| P1-2/3 | loading 计数器 + favoriteIds 级联清理 |

### 阶段四：架构清理
> 预估 3-5 天，可按需取舍

| 编号 | 任务 |
|------|------|
| P1-9 | Canvas 增量更新（diff 而非全量替换）|
| P1-11 | 相似度索引持久化 |
| P1-12 | FileTree 虚拟化 |
| P2-4 | 删除或重写 plugin_system |
| P2-5/6 | 统一相似度接口 + 拆分 watcher.rs |
| - | 补充后端单元测试 |
| - | 收紧 fs scope + 数据库迁移机制 |

---

## 附：工作量与优先级矩阵

```
高价值 ┃ P0-1,P0-5,P0-6   P0-3,P0-8
       ┃ (S/M, 立即)      (M, 阶段二)
       ┃
       ┃ P1-2,P1-5,P1-10  P1-4,P1-9,P1-11
       ┃ (S, 顺手做)       (L, 长期)
       ┃━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
       ┃ P3-1,P3-2        P2-4,P2-5
低价值 ┃ (S, 空闲做)       (L, 按需)
       ┃
       低成本            高成本
```

---

**文档版本**: 1.0
**下次评审**: 完成阶段一后
