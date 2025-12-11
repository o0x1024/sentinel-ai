# Sentinel AI License 功能使用指南

## 概述

Sentinel AI 使用基于 Ed25519 非对称签名的 License 系统来保护软件授权。该系统包含以下特性：

- 跨平台机器码采集（macOS/Windows/Linux）
- Ed25519 非对称签名验证
- 反调试检测
- 代码完整性校验
- 字符串加密
- 多关键功能点验证

## License 生成工具

### 编译

```bash
cd src-tauri
cargo build --release --package sentinel-license --bin license_generator
```

### 命令列表

```bash
./target/release/license_generator help
```

| 命令 | 说明 |
|------|------|
| `generate-keys` / `gen` | 生成新的 Ed25519 密钥对 |
| `sign <machine_id> [metadata]` | 为机器码签名生成 License |
| `verify <license_key>` | 验证 License 有效性 |
| `show-public-key` / `pubkey` | 显示当前公钥 |

## 使用流程

### 1. 首次配置（仅一次）

生成密钥对：

```bash
./target/release/license_generator gen
```

输出示例：
```
Generating new Ed25519 key pair...

Keys generated and saved to: license_keys.json

=== PUBLIC KEY (embed in application) ===
yzCNnuh1Mj0rXdWqvjvWRS6bxXp3Kw9GPu5gDDxrSsk=

=== PRIVATE KEY (keep secret!) ===
9ZaYsIyMqT/hFaHQpQ0XrSEBlH7AwQWtqDnf5ip0UXQ=

⚠️  WARNING: Keep the private key secure!
```

**重要**：
- `license_keys.json` 包含私钥，必须妥善保管
- 公钥已自动嵌入到 `sentinel-license/src/crypto.rs`

### 2. 获取用户机器码

用户在应用激活界面可以看到并复制机器码，格式为：
```
XXXX-XXXX-XXXX-XXXX
```

例如：`A1B2-C3D4-E5F6-0789`

**注意**：机器码必须是有效的十六进制字符（0-9, A-F）

### 3. 生成 License

```bash
./target/release/license_generator sign A1B2-C3D4-E5F6-0789 "Customer Name"
```

输出示例：
```
Signing license for machine ID: A1B2-C3D4-E5F6-0789

=== LICENSE KEY ===
eyJtYWNoaW5lX2lkIjoiYTFiMmMzZDRlNWY2MDc4OTAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMCIsInNpZ25hdHVyZSI6ImJvTVdYU0h5ejNHT0ZmQXpUaGEyTGdEMjVzeGpVSCthZzZCK3F3bjIwSC9JWk5OSUY4SjFpSXYzZDRhRW1CTVhKZzVJam9FSldkQ2ZIWWNYOFdiZURRPT0iLCJtZXRhZGF0YSI6IlRlc3QifQ==

Machine ID (full): a1b2c3d4e5f60789000000000000000000000000000000000000000000000000
Metadata: Customer Name
```

将生成的 License Key 发送给用户。

### 4. 验证 License（可选）

```bash
./target/release/license_generator verify "<license_key>"
```

输出示例：
```
Verifying license key...

✅ License is VALID
Machine ID: a1b2c3d4e5f60789000000000000000000000000000000000000000000000000
Metadata: Customer Name
```

### 5. 用户激活

用户在激活界面粘贴 License Key 并点击激活即可。

## 开发模式

- **Debug 模式**：自动跳过所有 License 验证，方便开发调试
- **Release 模式**：强制执行 License 验证

## License 验证点

以下功能在 Release 模式下需要有效 License：

| 功能 | 位置 |
|------|------|
| AI Agent 执行 | `commands/ai.rs` |
| RAG 查询 | `commands/rag_commands.rs` |
| RAG 辅助回答 | `commands/rag_commands.rs` |
| 工具执行 | `commands/tool_commands.rs` |
| 工作流执行 | `sentinel-workflow/commands.rs` |
| 被动扫描 | `commands/passive_scan_commands.rs` |

## 前端集成

前端组件 `LicenseActivation.vue` 提供：

- 显示机器码（可复制）
- License Key 输入框
- 激活按钮
- 错误提示

相关 Tauri 命令：

| 命令 | 说明 |
|------|------|
| `get_license_info` | 获取 License 状态和机器码 |
| `activate_license` | 激活 License |
| `check_license` | 检查是否已授权 |
| `get_machine_id` | 获取机器码（显示格式） |
| `get_machine_id_full` | 获取完整机器码哈希 |
| `deactivate_license` | 注销 License |

## 文件结构

```
src-tauri/
├── sentinel-license/
│   ├── src/
│   │   ├── lib.rs           # 主入口
│   │   ├── machine_id.rs    # 机器码采集
│   │   ├── crypto.rs        # 加密签名
│   │   ├── validator.rs     # License 验证
│   │   ├── storage.rs       # License 存储
│   │   ├── anti_debug.rs    # 反调试
│   │   ├── obfuscate.rs     # 字符串加密
│   │   ├── integrity.rs     # 代码完整性
│   │   └── bin/
│   │       └── license_generator.rs  # 生成工具
│   └── Cargo.toml
├── license_keys.json        # 密钥对文件（⚠️ 保密）
└── src/
    └── commands/
        └── license_commands.rs  # Tauri 命令
```

## 安全注意事项

1. **私钥保护**：`license_keys.json` 绝不能泄露或提交到版本控制
2. **公钥更换**：如需更换密钥对，需重新编译应用并重新为所有用户生成 License
3. **License 存储**：用户的 License 存储在 `~/Library/Application Support/sentinel-ai/.sentinel_auth`（macOS）
