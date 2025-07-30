---
layout: home

hero:
  name: "Sentinel AI"
  text: "AI驱动的漏洞挖掘平台"
  tagline: 智能化安全测试，高效漏洞发现
  image:
    src: /logo.svg
    alt: Sentinel AI
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: 查看源码
      link: https://github.com/user/sentinel-ai

features:
  - icon: 🤖
    title: AI驱动的智能扫描
    details: 利用先进的人工智能技术，自动生成扫描策略，智能分析漏洞，提供精准的风险评估和修复建议。
  
  - icon: 🔧
    title: MCP工具集成
    details: 集成Nuclei、Nmap、Subfinder等主流安全工具，通过MCP协议统一管理，提供标准化的工具调用接口。
  
  - icon: 📊
    title: 现代化界面
    details: 基于Vue3和DaisyUI构建的现代化用户界面，支持多主题切换，提供直观的数据可视化和用户体验。
  
  - icon: 🚀
    title: 高性能架构
    details: Tauri + Rust后端架构确保高性能和安全性，优化的构建流程和部署策略，支持跨平台运行。
  
  - icon: 💼
    title: 企业级功能
    details: 完整的项目管理、收益统计、任务调度和报告生成功能，满足专业安全测试团队的需求。
  
  - icon: 🔐
    title: 安全可靠
    details: 本地化部署，数据安全可控，完善的错误处理和日志记录，提供稳定可靠的服务。
---

## 快速了解

Sentinel AI 是一个现代化的AI驱动漏洞挖掘平台，旨在为安全研究人员和Bug Bounty猎人提供高效、智能的安全测试工具。

### 核心特性

- **🎯 智能扫描策略**: AI自动生成针对性扫描方案
- **⚡ 实时监控**: 扫描进度和结果实时展示
- **📈 数据分析**: 收益统计和趋势分析
- **🔄 自动化流程**: 从发现到报告的全流程自动化
- **🛠️ 工具集成**: 无缝集成主流安全工具

### 技术栈

| 前端 | 后端 | 数据库 | 工具 |
|------|------|--------|------|
| Vue3 + TypeScript | Rust + Tauri | SQLite | Nuclei, Nmap |
| DaisyUI + Tailwind | MCP协议 | 迁移系统 | Subfinder, Httpx |
| Chart.js | AI服务集成 | 索引优化 | 自定义脚本 |

### 开发进度

当前项目完成度：**87.5%** (7/8 阶段完成)

- ✅ 基础架构搭建
- ✅ MCP协议实现  
- ✅ 数据库设计
- ✅ AI服务集成
- ✅ 核心业务功能
- ✅ 界面完善与UX
- ✅ 性能优化与部署
- 🚧 测试与文档 (进行中)

### 开始使用

```bash
# 克隆项目
git clone https://github.com/user/sentinel-ai.git
cd sentinel-ai

# 安装依赖
npm install

# 开发模式
npm run tauri:dev

# 构建发布
npm run build:release
```

### 社区与支持

- 📖 [用户指南](/guide/introduction) - 详细的使用说明
- 🛠️ [开发文档](/development/architecture) - 技术架构和开发指南  
- 🚀 [部署指南](/deployment/production) - 生产环境部署
- 🐛 [问题反馈](https://github.com/user/sentinel-ai/issues) - Bug报告和功能请求
- 💬 [讨论区](https://github.com/user/sentinel-ai/discussions) - 社区交流

### 许可证

本项目基于 [MIT License](https://github.com/user/sentinel-ai/blob/main/LICENSE) 开源。 