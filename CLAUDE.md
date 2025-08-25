# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Sentinel AI 是一个 Tauri + Vue + TypeScript 应用程序，结合了 AI 驱动的漏洞挖掘与 MCP (Model Context Protocol) 集成。该应用具有多个 AI 代理架构、安全扫描工具和统一的工具管理系统。

## 常用命令

### 开发和构建
- `npm run dev` - 启动开发服务器
- `npm run build` - 构建应用程序
- `npm run build:check` - 运行 TypeScript 检查并构建
- `npm run build:release` - 构建生产版本
- `npm run tauri dev` - 运行 Tauri 开发模式
- `npm run tauri build --release` - 构建 Tauri 发布版本


### Rust 后端命令
- `cargo check` - 检查 Rust 代码
- `cargo clippy` - 运行 Clippy 代码检查
- `cargo fmt` - 格式化 Rust 代码
- `cargo test` - 运行 Rust 测试

## 架构概述

### 前端结构
- **Vue 3 + TypeScript** 使用组合式 API
- **Pinia** 用于状态管理
- **Vue Router** 用于导航
- **Tailwind CSS + DaisyUI** 用于样式
- **i18n** 用于国际化 (英文/中文)

### 后端结构
- **Tauri** 框架用于桌面应用程序
- **Rust** 后端使用异步运行时 (Tokio)
- **SQLite** 数据库配合 SQLx
- **MCP (Model Context Protocol)** 集成使用 `rmcp` 包

### 核心组件

#### AI 服务层 (`src-tauri/src/ai_adapter/`)
- 多提供商 AI 支持 (OpenAI, Anthropic, Gemini, DeepSeek 等)
- AI API 调用的统一 HTTP 适配器
- `providers/` 目录中的提供商特定实现
- 全局代理配置支持

#### 代理系统 (`src-tauri/src/engines/`)
- **LLM Compiler**: 任务规划和执行引擎
- **Plan and Execute**: 分层任务规划系统
- **ReWOO**: 无观察推理框架
- **Intelligent Dispatcher**: 智能查询路由
- 每个引擎都有自己的统一工具访问适配器

#### 工具管理 (`src-tauri/src/tools/`)
- **统一工具系统**: 集中化工具注册和执行
- **MCP 集成**: 支持外部 MCP 服务器和工具
- **内置安全工具**: 子域名扫描、端口扫描、漏洞检测
- **框架适配器**: 工具在所有代理架构中通用

#### 数据库层 (`src-tauri/src/database/`)
- SQLite 配合 SQLx ORM
- 数据访问的仓储模式
- 支持资产、漏洞、扫描会话、配置

#### 命令系统 (`src-tauri/src/commands/`)
- 按功能组织的 Tauri 命令
- AI 服务、MCP 工具、扫描、资产管理
- 性能监控和配置管理

### 核心特性

1. **多架构支持**: 应用程序支持多个 AI 代理架构，可根据任务需求选择
2. **统一工具接口**: 所有安全工具（内置和基于 MCP）都通过统一接口访问
3. **智能调度**: 自动将查询路由到适当的 AI 代理和工具
4. **实时监控**: 性能指标和执行跟踪
5. **持久化存储**: 数据库支持的配置、扫描结果和代理状态

## 开发工作流

### 添加新的 AI 提供商
1. 在 `src-tauri/src/ai_adapter/providers/` 中创建提供商实现
2. 实现 `AiProvider` trait
3. 在 `mod.rs` 中注册并更新提供商列表
4. 在 `src/components/Settings/AISettings.vue` 中添加 UI 组件

### 添加新工具
1. 在 `src-tauri/src/tools/` 中实现工具或创建 MCP 服务器
2. 通过 `unified_manager.rs` 向统一工具系统注册
3. 在适当的 Vue 组件中添加前端集成
4. 更新工具类别和元数据

### 使用代理架构
1. 架构引擎位于 `src-tauri/src/engines/`
2. 每个引擎实现 `EngineAdapter` trait
3. 工具通过全局工具系统访问
4. 代理会话在 `src-tauri/src/agents/` 中管理

### 数据库架构变更
1. 在 `src-tauri/src/models/` 中更新模型
2. 在 `src-tauri/src/database/` 中修改 DAO 实现
3. 在 `src-tauri/src/services/` 中更新服务层
4. 在 `src-tauri/src/commands/` 中添加 Tauri 命令

## 测试策略

- **单元测试**: 使用 Vitest 进行单独组件测试
- **集成测试**: API 和服务层测试
- **端到端测试**: 使用 Playwright 进行完整应用程序测试
- **Rust 测试**: 使用 cargo test 进行后端逻辑测试
- **性能测试**: 监控和优化验证

## 配置

- 应用设置存储在 SQLite 数据库中
- 环境变量用于开发配置
- MCP 服务器配置通过工具模块管理
- AI 提供商配置存储在数据库中，凭据加密

## 安全考虑

- 内置安全工具仅用于防御目的
- 漏洞扫描专注于检测和报告
- MCP 连接经过验证和沙盒处理
- 敏感数据在静态存储时尽可能加密