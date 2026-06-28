---
title: 模型方案设计
description: 小当家项目的 AI 模型方案设计文档
---

# 模型方案设计

## 1. 方案概述

小当家的 AI 能力采用本地化部署方案，结合规则引擎与轻量级模型，在保护用户隐私的同时提供智能文件管理功能。

## 2. 技术架构

### 2.1 分层架构

```
+------------------------------------------+
+           应用层 (Application)           ++
+  文件管理 / 画布交互 / 用户界面          ++
+------------------------------------------+
+           意图识别层 (Intent)            ++
+  查询理解 / 任务分类 / 参数提取          ++
+------------------------------------------+
+           能力引擎层 (Engine)            ++
+  关联分析 / 标签生成 / 相似度计算        ++
+------------------------------------------+
+           数据服务层 (Data)              ++
+  SQLite / 向量存储 / 文件索引            ++
+------------------------------------------+
```

## 3. 核心模型设计

### 3.1 标签生成模型

**输入**: 文件元数据 (名称、扩展名、大小、路径)

**输出**: 标签集合

**实现方案**: 规则引擎 + 关键词匹配

```rust
// 基于扩展名的标签映射
let ext_tags = match extension.lowercase().as_str() {
    "js" | "jsx" | "ts" | "tsx" => vec!["JavaScript", "前端"],
    "py" => vec!["Python", "后端"],
    "rs" => vec!["Rust", "系统编程"],
    "json" | "yaml" | "toml" => vec!["配置文件"],
    "md" => vec!["文档", "Markdown"],
    "png" | "jpg" | "jpeg" | "gif" => vec!["图片", "图像"],
    _ => vec![],
};

// 基于文件名的关键词匹配
let name_tags = match name.to_lowercase().as_str() {
    n if n.contains("test") || n.contains("spec") => vec!["测试"],
    n if n.contains("config") => vec!["配置"],
    n if n.starts_with('.') => vec!["隐藏文件"],
    _ => vec![],
};

// 基于大小的标签
let size_tags = match size {
    s if s > 100 * 1024 * 1024 => vec!["超大文件"],
    s if s > 10 * 1024 * 1024 => vec!["大文件"],
    _ => vec![],
};
```

### 3.2 关联分析模型

**输入**: 文件节点集合

**输出**: 文件关联关系 (relation_type, confidence)

**关联类型**:

| 类型 | 描述 | 计算方法 | 当前置信度 |
|------|------|---------|-----------|
| `import` | 导入/依赖关系 | 解析代码 import/require 语句 (ast_parser.rs) | 0.8-0.9 |
| `similar` | 命名/风格相似 | 文件名包含关系 + 样式文件匹配 | 0.5-0.7 |
| `auto` | 自动检测 | 同扩展名/路径引用 | 0.4-0.6 |
| `same_dir` | 同目录 | 父目录相同 | 待实现 |
| `same_ext` | 同类型 | 扩展名相同 | 合并到 auto |
| `created_together` | 同时创建 | 创建时间差 < 1分钟 | 待实现 |

**置信度计算**:

```rust
fn calculate_confidence(relation_type: &str, evidence: &str) -> f64 {
    match relation_type {
        "import" => 0.9,           // 高置信度
        "reference" => 0.7,        // 中高置信度
        "similar_name" => 0.5,     // 中等置信度
        "same_dir" => 0.3,         // 低置信度
        "same_ext" => 0.2,         // 较低置信度
        _ => 0.1,
    }
}
```

### 3.3 相似度计算模型

**输入**: 目标文件、候选文件集合

**输出**: 相似度排序列表

**计算方法**:

```rust
fn calculate_similarity(file1: &FileNode, file2: &FileNode) -> f64 {
    let mut score = 0.0;

    // 1. 扩展名相似度 (权重: 0.3)
    if file1.extension == file2.extension {
        score += 0.3;
    }

    // 2. 目录邻近度 (权重: 0.25)
    let dir_similarity = calculate_dir_proximity(file1, file2);
    score += 0.25 * dir_similarity;

    // 3. 命名相似度 (权重: 0.2)
    let name_similarity = levenshtein_distance(&file1.name, &file2.name);
    score += 0.2 * (1.0 - name_similarity);

    // 4. 标签重叠度 (权重: 0.15)
    let tag_overlap = calculate_tag_overlap(&file1.tags, &file2.tags);
    score += 0.15 * tag_overlap;

    // 5. 大小相近度 (权重: 0.1)
    let size_similarity = calculate_size_similarity(file1.size, file2.size);
    score += 0.1 * size_similarity;

    score
}
```

### 3.4 搜索模型

**输入**: 用户查询文本

**输出**: 匹配的文件列表

**搜索策略**:

1. **精确匹配**: 文件名完全匹配
2. **模糊匹配**: 文件名包含查询词
3. **标签匹配**: 查询词匹配标签
4. **路径匹配**: 查询词匹配文件路径
5. **扩展名匹配**: 查询词匹配扩展名

**排序策略**:

```
结果排序 = 精确匹配 * 1.0 + 模糊匹配 * 0.7 + 标签匹配 * 0.5 + 路径匹配 * 0.3
```

## 4. 向量搜索方案 (当前状态)

### 4.1 方案对比

| 方案 | 优点 | 缺点 | 状态 |
|------|------|------|------|
| 规则引擎 + 语义搜索 | 隐私保护、低延迟、零依赖 | 准确性有限 | ✅ 已实现 |
| 轻量语义嵌入 | 本地运行、32维向量、cosine similarity | 非深度学习嵌入 | ✅ 已实现 |
| TF-IDF 内容相似度 | 基于文件内容、Jaccard/余弦相似 | 需读取文件内容 | ✅ 已实现 |
| Chroma | 开源、易用 | 需额外安装 | ⏳ 规划中 |
| Milvus | 功能强大 | 资源消耗大 | ⏳ 规划中 |

### 4.2 已实现阶段

1. **阶段一**: 基于关键词的搜索 ✅ (watcher.rs)
2. **阶段一+**: 语义搜索 (同义词扩展) ✅ (semantic_search.rs)
3. **阶段二**: TF-IDF 内容相似度 ✅ (tfidf.rs)
4. **阶段二+**: 轻量语义嵌入 ✅ (semantic_embedding.rs)
5. **阶段二+**: AST 导入关系解析 ✅ (ast_parser.rs)
6. **阶段三**: Chroma 向量库集成 ⏳ (规划中)

## 5. 性能优化

### 5.1 缓存策略

- 标签结果缓存: 首次生成后持久化
- 关联分析结果缓存: 按项目缓存
- 搜索结果缓存: LRU 缓存，1000 条上限

### 5.2 增量更新

- 文件变更时仅更新相关节点
- 关联分析支持增量计算
- 后台异步处理，不阻塞 UI

### 5.3 并行处理

- 文件扫描: 多线程遍历
- 标签生成: 并行处理
- 相似度计算: 批量计算

## 6. 扩展性设计

### 6.1 模型插件化

```rust
trait AIModel {
    fn analyze(&self, input: &Input) -> Output;
    fn name(&self) -> &str;
}

// 注册自定义模型
registry.register("tagger", Box::new(RuleBasedTagger));
registry.register("similarity", Box::new(FeatureSimilarity));
```

### 6.2 API 设计

未来可接入外部 AI 服务:

```rust
#[tauri::command]
async fn enhance_with_ai(project_id: String, file_id: String, provider: String) -> Result<AIEnhancement, String> {
    match provider.as_str() {
        "openai" => openai_enhance(file_id).await,
        "local" => local_model_enhance(file_id),
        _ => Err("Unknown provider".to_string()),
    }
}
```

---

*最后更新: 2026-06-16*
