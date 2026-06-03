
# Sentinel AI Community Edition

> AI 驱动的桌面安全分析平台 — 开源 Community 版（[AGPL-3.0](LICENSE)）

**Community Edition** 包含流量分析、AI 助手、工作流、RAG 知识库、插件系统与工具生态。**漏洞赏金（Bug Bounty）模块不包含在本版本**，该功能仅在 [Sentinel AI Pro](https://github.com/o0x1024/sentinel-ai-pro) 商业版中提供。

![Sentinel AI 应用运行截图](public/image.png)

## 开源协议

本仓库以 **[GNU Affero General Public License v3.0（AGPL-3.0）](LICENSE)** 发布。

- 你可以自由使用、修改和分发本软件。
- 若你修改本软件并**通过网络提供服务**，或**分发修改后的版本**，须按 AGPL-3.0 公开相应源代码。
- **漏洞赏金模块不在本仓库中**，完整商业功能见 [Sentinel AI Pro](https://github.com/o0x1024/sentinel-ai-pro)（单独授权）。

完整协议文本见仓库根目录 [LICENSE](LICENSE) 文件。

## 联系与 Pro 版咨询

如需开通 **Sentinel AI Pro** 或咨询漏洞赏金模块，请微信扫码添加：

<p align="center">
  <img src="public/community/wechat-qrcode.png" alt="微信二维码" width="220" />
</p>

应用内也可在侧边栏灰色 **漏洞赏金 / PRO** 入口，或 **帮助中心 → 漏洞赏金（Pro 专属）** 查看同一二维码。

## 核心能力

| 模块 | 说明 |
| --- | --- |
| 流量分析 | 代理监听、HTTPS 解密、拦截、重放、findings |
| AI 助手 | 多轮对话、工具调用、子代理、角色策略 |
| 工作流 | 可视化编排、调度运行、运行记录 |
| 知识库 RAG | 文档入库、向量检索、检索增强问答 |
| 应用工具 / MCP | 内置工具、MCP Server、交互终端 |
| 插件系统 | 插件开发、测试、审核、启停 |
| 系统设置 | AI、数据库、代理、RAG 等全局配置 |

## 技术栈

Tauri v2 · Vue 3 · TypeScript · Rust · SQLite

## 快速开始

**环境要求：** Node.js 18+、Rust stable、Tauri v2 构建依赖

1. 安装依赖：`npm install`
2. 前端开发：`npm run dev`
3. 桌面开发：`npm run tauri dev`
4. 构建发布：`npm run build` / `npm run build:release`

## 下载

预编译安装包见 [Releases](https://github.com/o0x1024/sentinel-ai-community/releases)。

## 发布（Maintainers）

| Workflow | 触发条件 |
| --- | --- |
| [CI](.github/workflows/ci.yml) | push / PR 到 `main` |
| [Build and Release](.github/workflows/build-and-release.yml) | 推送 tag `v*` |

推送 tag（例如 `v0.1.0`）到本仓库即可触发 Release 构建。可选配置 `TAURI_SIGNING_*`、`MACOS_CERTIFICATE` 等 Secrets 以启用自动更新签名；未配置时仍会上传未签名的 `.dmg` / `.exe` 安装包。
