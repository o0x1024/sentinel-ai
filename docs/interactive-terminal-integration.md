# Interactive Terminal Panel Integration

## 概述

已成功将交互式终端（Interactive Terminal）集成到 AgentView 中，作为一个独立的侧边面板，类似于 Vision Explorer 和 Todo Panel。

## 实现内容

### 1. 创建 useTerminal Composable (`src/composables/useTerminal.ts`)

管理终端面板的全局状态：

- **状态管理**:
  - `isTerminalActive`: 终端面板是否激活
  - `currentSessionId`: 当前终端会话 ID
  - `hasHistory`: 是否有终端历史记录

- **操作方法**:
  - `openTerminal(sessionId?)`: 打开终端面板
  - `closeTerminal()`: 关闭终端面板
  - `toggleTerminal()`: 切换终端面板
  - `setSessionId(sessionId)`: 设置会话 ID
  - `clearTerminal()`: 清空终端状态
  - `resetTerminal()`: 重置终端（关闭并清空）

### 2. 更新 InteractiveTerminal.vue

将终端组件适配为面板模式：

**UI 改进**:
- 添加面板头部，包含标题、会话信息和操作按钮
- 添加状态栏，显示连接状态和会话 ID
- 简化样式，使用 DaisyUI 组件
- 添加关闭按钮，触发 `close` 事件

**功能保持**:
- WebSocket 连接管理
- xterm.js 终端渲染
- 命令输入和输出
- 自动重连和错误处理

### 3. 集成到 AgentView

**导入组件**:
```typescript
import { useTerminal } from '@/composables/useTerminal'
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
```

**状态管理**:
```typescript
const terminalComposable = useTerminal()
const isTerminalActive = computed(() => terminalComposable.isTerminalActive.value)
const hasTerminalHistory = computed(() => terminalComposable.hasHistory.value)
```

**添加切换按钮**:
在头部添加终端按钮，仅在有历史记录时显示：
```vue
<button 
  v-if="hasTerminalHistory"
  @click="terminalComposable.toggleTerminal()"
  class="btn btn-sm gap-1"
  :class="isTerminalActive ? 'btn-primary' : 'btn-ghost text-primary'"
>
  <i class="fas fa-terminal"></i>
  <span>{{ t('agent.terminal') }}</span>
</button>
```

**侧边面板集成**:
```vue
<div class="sidebar-container" v-if="isVisionActive || isTodosPanelActive || isTerminalActive">
  <VisionExplorerPanel v-if="isVisionActive" ... />
  <TodoPanel v-else-if="isTodosPanelActive" ... />
  <InteractiveTerminal v-else-if="isTerminalActive" @close="handleCloseTerminal" />
</div>
```

### 4. 国际化支持

**中文 (`src/i18n/locales/agent/zh.ts`)**:
```typescript
terminalPanelOpen: '终端面板已打开',
viewTerminal: '查看交互式终端',
terminal: '终端',
interactiveTerminal: '交互式终端',
clear: '清空',
reconnect: '重新连接',
disconnect: '断开连接',
```

**英文 (`src/i18n/locales/agent/en.ts`)**:
```typescript
terminalPanelOpen: 'Terminal Panel Open',
viewTerminal: 'View Interactive Terminal',
terminal: 'Terminal',
interactiveTerminal: 'Interactive Terminal',
clear: 'Clear',
reconnect: 'Reconnect',
disconnect: 'Disconnect',
```

## 使用方式

### 1. 打开终端面板

```typescript
import { useTerminal } from '@/composables/useTerminal'

const terminal = useTerminal()

// 打开终端
terminal.openTerminal()

// 打开终端并指定会话 ID
terminal.openTerminal('session-id-123')
```

### 2. 在 Agent 中使用

当 Agent 需要使用交互式终端时：

1. 调用 `terminal.openTerminal()` 显示终端面板
2. 用户可以在终端中执行命令
3. 终端保持会话状态，支持持续交互
4. 点击关闭按钮或调用 `terminal.closeTerminal()` 关闭面板

### 3. 面板切换

- Vision Explorer、Todo Panel 和 Terminal Panel 是互斥的
- 同一时间只能显示一个侧边面板
- 点击对应按钮可以在不同面板之间切换

## 应用场景

交互式终端特别适用于需要持续交互的安全测试工具：

### 1. Metasploit Framework
```bash
msfconsole
use exploit/multi/handler
set PAYLOAD windows/meterpreter/reverse_tcp
set LHOST 192.168.1.100
exploit
```

### 2. 数据库交互
```bash
mysql -u root -p
USE database;
SELECT * FROM users;
```

### 3. 反向 Shell
```bash
# 获得目标机器的 shell 后持续执行命令
whoami
uname -a
cat /etc/passwd
```

### 4. 容器操作
```bash
docker exec -it container bash
cd /app
ls -la
```

### 5. 实时监控
```bash
tail -f /var/log/access.log
tcpdump -i eth0 -n
```

## 技术架构

```
┌─────────────────────────────────────────────────────────┐
│                      AgentView                          │
│  ┌────────────────────────────────────────────────┐    │
│  │  Header (with Terminal button)                 │    │
│  └────────────────────────────────────────────────┘    │
│  ┌──────────────────┬────────────────────────────┐    │
│  │                  │  Sidebar (350px)           │    │
│  │  MessageFlow     │  ┌──────────────────────┐  │    │
│  │                  │  │ InteractiveTerminal  │  │    │
│  │                  │  │  - Panel Header      │  │    │
│  │                  │  │  - Status Bar        │  │    │
│  │                  │  │  - xterm.js          │  │    │
│  │                  │  │  - WebSocket         │  │    │
│  │                  │  └──────────────────────┘  │    │
│  └──────────────────┴────────────────────────────┘    │
│  ┌────────────────────────────────────────────────┐    │
│  │  InputArea                                     │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
                ┌─────────────────────┐
                │  useTerminal        │
                │  (Global State)     │
                └─────────────────────┘
                          │
                          ▼
                ┌─────────────────────┐
                │  Terminal Server    │
                │  (WebSocket)        │
                └─────────────────────┘
                          │
                          ▼
                ┌─────────────────────┐
                │  Docker Container   │
                │  (Kali Linux)       │
                └─────────────────────┘
```

## 后续优化建议

1. **自动打开终端**: 当 Agent 调用需要交互的工具时，自动打开终端面板
2. **多会话管理**: 支持同时管理多个终端会话，可以在不同会话间切换
3. **会话持久化**: 保存终端会话历史，刷新页面后可以恢复
4. **命令建议**: 根据上下文提供命令自动补全和建议
5. **输出高亮**: 对特殊输出（错误、警告、成功）进行颜色高亮
6. **快捷键支持**: 添加键盘快捷键快速打开/关闭终端

## 相关文件

- `src/composables/useTerminal.ts` - 终端状态管理
- `src/components/Tools/InteractiveTerminal.vue` - 终端组件
- `src/components/Agent/AgentView.vue` - Agent 视图集成
- `src/i18n/locales/agent/zh.ts` - 中文翻译
- `src/i18n/locales/agent/en.ts` - 英文翻译
- `src-tauri/sentinel-tools/src/terminal/` - 后端终端服务器
- `src-tauri/src/commands/terminal_commands.rs` - Tauri 命令

## 测试建议

1. **基本功能测试**:
   - 点击终端按钮打开/关闭面板
   - 在终端中执行简单命令（如 `whoami`, `pwd`, `ls`）
   - 测试命令输出显示是否正常

2. **交互测试**:
   - 测试需要持续交互的命令（如 `top`, `tail -f`）
   - 测试 Ctrl+C 中断命令
   - 测试多行命令输入

3. **会话管理**:
   - 测试断开重连
   - 测试会话状态保持
   - 测试错误处理

4. **UI 测试**:
   - 测试面板切换（Vision/Todo/Terminal）
   - 测试响应式布局
   - 测试深色主题显示

5. **性能测试**:
   - 测试大量输出的性能
   - 测试长时间运行的命令
   - 测试内存占用
