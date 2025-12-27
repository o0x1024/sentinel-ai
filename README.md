# Sentinel AI - Security Analysis Platform

A comprehensive security analysis platform built with Tauri, Vue 3, and Rust.

## Features

- 🔍 **Traffic Analysis**: HTTP/HTTPS proxy with MITM capabilities
- 📦 **Packet Capture**: Network packet capture and analysis (Wireshark-like)
- 🤖 **AI-Powered Analysis**: Multi-agent AI system for vulnerability detection
- 🔌 **Plugin System**: Extensible plugin architecture with JavaScript/TypeScript support
- 📊 **RAG System**: Retrieval-Augmented Generation for security knowledge
- 🔐 **Security Center**: Comprehensive security scanning and reporting

## Windows Packet Capture Setup

For packet capture functionality on Windows, you need to install **Npcap**:

1. Download Npcap from: https://nmap.org/npcap/
2. During installation, **enable "Install Npcap in WinPcap API-compatible Mode"**
3. Restart your computer after installation

For detailed setup instructions, see: [Windows Packet Capture Setup Guide](src-tauri/docs/windows_packet_capture_setup.md)

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Type Support For `.vue` Imports in TS

Since TypeScript cannot handle type information for `.vue` imports, they are shimmed to be a generic Vue component type by default. In most cases this is fine if you don't really care about component prop types outside of templates. However, if you wish to get actual prop types in `.vue` imports (for example to get props validation when using manual `h(...)` calls), you can enable Volar's Take Over mode by following these steps:

1. Run `Extensions: Show Built-in Extensions` from VS Code's command palette, look for `TypeScript and JavaScript Language Features`, then right click and select `Disable (Workspace)`. By default, Take Over mode will enable itself if the default TypeScript extension is disabled.
2. Reload the VS Code window by running `Developer: Reload Window` from the command palette.

You can learn more about Take Over mode [here](https://github.com/johnsoncodehk/volar/discussions/471).
