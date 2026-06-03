
# Sentinel AI Community Edition

[中文](./README.md) | **English**

> AI-powered desktop security analysis platform — open-source Community Edition

The **Community Edition** includes traffic analysis, AI assistant, workflows, RAG knowledge base, plugin system, and tool ecosystem. The **Bug Bounty module is not included** in this edition; it is available in [Sentinel AI Pro](https://github.com/o0x1024/sentinel-ai-pro) only.

Sentinel AI is built for security research and offensive/defensive engineering teams. Rather than being a single-point tool, it connects **traffic analysis**, **AI assistant**, **workflow automation**, **RAG knowledge base**, **plugin system**, and **tool ecosystem (MCP / built-in tools)** into a continuously evolving analysis loop.

![Sentinel AI screenshot](public/image.png)

## Why Sentinel AI

Common problems with traditional security toolchains:

- Scattered data: traffic, vulnerabilities, assets, notes, and scripts live in separate systems
- Broken automation: verification, retesting, notification, and archival still rely on manual handoffs after discovery
- Weak AI adoption: AI can answer questions but cannot reliably invoke internal capabilities

Sentinel AI is designed to:

- Make traffic and scan results directly consumable by AI as context
- Turn plugin capabilities into tools AI can call, not isolated scripts
- Let high-frequency analysis flows become schedulable workflows

## Core Capabilities

| Module | Capabilities | Entry |
| --- | --- | --- |
| Traffic Analysis | Proxy, HTTPS decryption, HTTP/WS history, intercept, replay, findings | Traffic Analysis |
| AI Assistant | Multi-turn chat, tool use, sub-agent tracing, role policies, session management | AI Assistant |
| Workflow Studio | Visual orchestration, node execution, scheduling, run history | Workflow Studio |
| RAG Management | Document ingestion, vector search, collections, retrieval-augmented Q&A | RAG Management |
| Tools / MCP | Built-in tools, MCP servers, plugin tools, interactive terminal | Tools |
| Plugin Management | Plugin development, testing, review, enable/disable, store install & updates | Plugin Management |
| Settings | Global AI, database, network, proxy, RAG, licensing configuration | Settings |

## Highlight: Plugin System × AI Assistant

This is a core differentiator of Sentinel AI.

### 1. Plugins as governed capability units, not add-ons

Plugins fall into two main categories:

- **Traffic plugins**: attached to the traffic inspection pipeline to analyze HTTP/WS data and produce findings
- **Agent tool plugins**: registered in the unified tool layer for direct use by the AI assistant and workflows

From day one, plugins support:

- Lifecycle management (develop, test, review, publish, disable)
- Observability (execution records, result feedback, error tracing)
- Orchestration (reusable by AI and workflows)

### 2. AI assistant invokes plugin capabilities directly

The AI assistant is not only for Q&A—it performs real actions through tool calls:

- Read and analyze traffic and findings
- Invoke plugins for targeted detection or data processing
- Chain MCP and built-in tools for upstream/downstream actions
- Combine RAG knowledge to produce reusable analysis conclusions

### 3. Recommended closed-loop workflow

1. **Discover on traffic side**: traffic plugins emit structured findings
2. **Analyze with AI**: AI invokes plugin tools for verification, attribution, and correlation
3. **Automate with workflows**: mature response logic becomes scheduled workflows
4. **Persist in knowledge base**: conclusions are stored for reuse on similar issues

Together this upgrades the manual chain of discover → assess → execute → review into a sustainable automation loop.

## Module Overview

### Traffic Analysis

- Proxy, intercept, and replay for HTTP/HTTPS/WS analysis
- Real-time findings from traffic plugins feed downstream workflows
- Works with Proxifier and packet-capture setups for different network scenarios

### AI Assistant

- Multi-turn contextual sessions with role policies and tool-call traces
- Sub-agent execution with result handoff for complex task decomposition
- Consumes traffic, assets, knowledge base, and tool outputs directly

### Workflow Studio

- Node-based flows for high-frequency analysis patterns
- Save, run, stop, schedule, and inspect run history
- Reuses plugin tools and MCP tools for cross-system automation

### RAG Management

- Document import, chunking, embedding, and retrieval
- Traceable knowledge-augmented context for the AI assistant
- Stores playbooks, cases, and reference material

### Tools / MCP

- Unified management of built-in, plugin, workflow, and MCP tools
- MCP server integration with tool enumeration and invocation
- Built-in terminal for execution and verification in one workspace

#### Built-in Tools

- **Port scan**: basic port probing for exposure assessment; callable from AI assistant or workflow nodes
- **HTTP request**: agent-side HTTP(S) requests with custom method, headers, and body for reproduction and API checks
- **Local time**: current local time for time-aware reasoning and scheduling
- **Memory / context**: explicit long-term memory read/write for multi-turn tasks
- **OCR**: text extraction from screenshots and images for downstream analysis
- **Sub-agents & parallel execution**: spawn, wait, and aggregate sub-tasks within one job
- **Sub-agent state & events**: shared state and event bus between sub-agents
- **Tenth-man review**: adversarial review to challenge assumptions and surface blind spots
- **Todos**: explicit TODO lists in conversation with live UI panel
- **Web search**: controlled web search for external context
- **Skills**: dynamic skill listing and loading to extend agent capabilities

### Shell / Interactive Terminal / Docker Sandbox

- **One-shot Shell tool**: exposed via the AI tool layer for single commands; prefers Docker sandbox, falls back to host with safety policies
- **Interactive terminal**: PTY sessions for SSH, DB clients, REPLs, etc., with resize, history replay, and shared UI/LLM access
- **Docker sandbox image**: unified sandbox for Shell and terminal with minimal / kali / kali-full variants

### Plugin Management

- Full lifecycle from development through review to enable/disable
- Two execution surfaces: traffic inspection and agent tools
- Plugin capabilities reused by AI and workflows without silos

### Settings

- Central AI, database, proxy, RAG, security, and licensing configuration
- Supports team deployment, migration, and policy governance

## Tech Stack

- **Desktop**: Tauri v2
- **Frontend**: Vue 3 + Vite + TypeScript + Pinia + Vue Router + Tailwind/daisyUI
- **Backend**: Rust workspace
- **Storage**: SQLite

Key backend crates: sentinel-traffic, sentinel-plugins, sentinel-tools, sentinel-workflow, sentinel-rag, sentinel-llm, sentinel-db.

## Quick Start

### Requirements

- Node.js 18+
- Rust stable toolchain
- Tauri v2 build dependencies for your OS

### Local Development

1. Install dependencies: npm install
2. Frontend dev: npm run dev
3. Desktop dev: npm run tauri dev
4. Build: npm run build or npm run build:release

### Development & Testing

- Type check: npm run type-check
- Unit/integration tests: npm run test, npm run test:unit, npm run test:integration
- E2E: npm run test:e2e
- Rust check: cargo check in src-tauri

## Project Layout

- **src/**: Vue frontend (views, components, services)
- **src-tauri/**: Tauri + Rust workspace (commands, sentinel-* crates)
- **plugins/**: plugin directory
- **scripts/**: development and build scripts

## Downloads

Pre-built installers: [Releases](https://github.com/o0x1024/sentinel-ai-community/releases).

## Release (Maintainers)

Community builds are published via GitHub Actions:

| Workflow | Trigger | Description |
| --- | --- | --- |
| [CI](.github/workflows/ci.yml) | push/PR to main | type-check + cargo check |
| [Build and Release](.github/workflows/build-and-release.yml) | tag push v* | macOS / Windows installers to Releases |

Push a version tag to trigger a release. Optional Tauri signing and macOS certificate secrets enable auto-update artifacts; without them, unsigned .dmg / .exe installers are still uploaded.

## Use Cases

- Daily traffic triage and vulnerability verification for security teams
- Automated asset and risk analysis in red/blue exercises
- Retest, notification, and archival automation in vulnerability operations
- Capturing team expertise as plugins and knowledge for reuse

## Contact & Pro Edition

To purchase **Sentinel AI Pro** or ask about the Bug Bounty module, scan the WeChat QR code:

<p align="center">
  <img src="public/community/wechat-qrcode.png" alt="WeChat QR code" width="220" />
</p>

You can also find the same QR code in the app under the gray **Bug Bounty / PRO** sidebar entry, or in **Help Center → Bug Bounty (Pro only)**.

## License

This repository is released under the **[GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE)**.

- You may use, modify, and distribute this software freely
- If you modify this software and **offer it as a network service**, or **distribute modified versions**, you must release the corresponding source code under AGPL-3.0
- The **Bug Bounty module is not in this repository**; the full commercial product is [Sentinel AI Pro](https://github.com/o0x1024/sentinel-ai-pro) (separate license)

See [LICENSE](LICENSE) in the repository root for the full license text.
