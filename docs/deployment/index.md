---
title: 部署与运维手册
description: 小当家项目的部署与运维指南
---

# 部署与运维手册

## 1. 开发环境搭建

### 1.1 基础环境要求

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| Node.js | ≥18.0.0 | 前端运行时 |
| npm | ≥9.0.0 | 包管理器 |
| Rust | ≥1.70.0 | 后端运行时 |
| Cargo | 与 Rust 版本匹配 | Rust 包管理器 |

### 1.2 安装步骤

**Windows 环境**:

```powershell
# 1. 安装 Node.js
# 访问 https://nodejs.org/ 下载 LTS 版本

# 2. 安装 Rust
# 访问 https://rust-lang.org/learn/get-started
winget install Rustlang.Rust.MSVC

# 3. 验证安装
node --version    # 应显示 v18.x.x 或更高
npm --version     # 应显示 9.x.x 或更高
rustc --version   # 应显示 1.70.0 或更高
cargo --version   # 应显示对应版本
```

**克隆项目**:

```powershell
# 克隆项目
git clone https://github.com/your-repo/xiaodangjia.git
cd xiaodangjia

# 安装前端依赖
npm install
```

### 1.3 开发命令

```powershell
# 开发模式 (前端 + 后端)
npm run tauri dev

# 仅运行前端
npm run dev

# 构建前端
npm run build
```

## 2. 生产环境构建

### 2.1 构建前检查

1. 确认所有测试通过
2. 更新版本号
3. 检查依赖安全性

### 2.2 构建步骤

```powershell
# 构建桌面应用
npm run tauri build
```

构建产物位于:
- Windows: `src-tauri/target/release/bundle/msi/`
- 便携版: `src-tauri/target/release/`

### 2.3 构建配置

在 `src-tauri/tauri.conf.json` 中配置:

```json
{
  "productName": "小当家",
  "version": "0.1.0",
  "identifier": "com.xiaodangjia.app",
  "app": {
    "windows": [{
      "title": "小当家 - 脑图式文件管理器",
      "width": 1400,
      "height": 900,
      "minWidth": 800,
      "minHeight": 600
    }]
  }
}
```

## 3. 安装与分发

### 3.1 Windows 安装

1. 下载 `.msi` 安装包
2. 双击运行安装向导
3. 选择安装目录
4. 完成安装

### 3.2 便携版使用

1. 解压 `.zip` 包
2. 运行 `xiaodangjia.exe`

### 3.3 依赖项

- **WebView2**: Windows 10/11 已内置，无需额外安装
- 如遇问题，下载安装: [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## 4. 数据管理

### 4.1 数据存储位置

| 数据类型 | 位置 |
|----------|------|
| SQLite 数据库 | `%APPDATA%/com.xiaodangjia.app/xiaodangjia.db` |
| 日志文件 | `%APPDATA%/com.xiaodangjia.app/logs/` |
| 配置文件 | `%APPDATA%/com.xiaodangjia.app/config.json` |

### 4.2 数据备份

定期备份数据目录:

```powershell
# 备份命令
Copy-Item -Recurse "$env:APPDATA%\com.xiaodangjia.app" ".\backup\xiaodangjia_backup"
```

### 4.3 数据恢复

1. 关闭应用
2. 替换数据目录
3. 重新启动应用

## 5. 运维监控

### 5.1 日志查看

日志文件位置: `%APPDATA%/com.xiaodangjia.app/logs/`

日志级别: ERROR, WARN, INFO, DEBUG

### 5.2 常见问题处理

**问题 1**: 应用启动失败

```
症状: 双击应用无反应
解决: 检查 WebView2 是否安装
```

**问题 2**: 项目加载失败

```
症状: 打开项目无文件显示
解决: 检查根目录路径是否存在
```

**问题 3**: 数据库损坏

```
症状: 应用异常退出后数据丢失
解决: 删除数据库文件后重新创建项目
```

## 6. 更新维护

### 6.1 版本检查

定期检查 GitHub releases 获取更新:
https://github.com/your-repo/xiaodangjia/releases

### 6.2 更新步骤

1. 备份当前数据
2. 下载新版本安装包
3. 覆盖安装或替换可执行文件

### 6.3 回滚操作

如需回滚:
1. 卸载新版本
2. 恢复备份数据
3. 安装旧版本

## 7. 安全配置

### 7.1 权限配置

应用权限在 `src-tauri/capabilities/default.json` 中定义:

```json
{
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default"
  ]
}
```

### 7.2 文件访问限制

应用仅能访问用户授权的目录，建议:
- 不授予系统敏感目录访问权限
- 定期审查已授权的目录

---

*最后更新: 2026-06-10*
