# 交互式终端显示修复

## 问题描述

LLM 执行 `interactive_shell` 命令后，虽然结果已返回给 LLM，但**右侧终端面板没有任何显示**。

## 原因分析

这是一个典型的**前端时序问题**：

1. LLM 工具执行完毕，触发 `agent:tool_call_complete` 事件
2. `useAgentEvents` 调用 `terminal.openTerminal()` 和 `terminal.writeToTerminal()`
3. `InteractiveTerminal` 组件开始挂载（Mounting）
4. `writeToTerminal` 发送消息时，`InteractiveTerminal` 可能还未完成挂载，尚未注册监听器
5. 导致消息被丢弃，终端面板虽然打开了，但是空的

## 解决方案：消息缓冲机制

我实现了一个消息缓冲机制，确保在终端组件准备好之前发送的消息不会丢失。

### 1. 修改 `useTerminal.ts`

- 添加全局 `messageBuffer` 数组
- 修改 `writeToTerminal`：如果没有注册的监听器，将消息存入缓冲区
- 修改 `onTerminalWrite`：当监听器注册时，立即刷新缓冲区中的所有消息

```typescript
// Buffer for messages sent before terminal is ready
const messageBuffer: string[] = []

function writeToTerminal(content: string) {
  if (terminalWriteCallbacks.size === 0) {
    // No active terminal, buffer the message
    messageBuffer.push(content)
    return
  }
  // ... dispatch to listeners
}

function onTerminalWrite(callback: (content: string) => void) {
  terminalWriteCallbacks.add(callback)
  
  // Flush buffer immediately
  if (messageBuffer.length > 0) {
    messageBuffer.forEach(msg => callback(msg))
    messageBuffer.length = 0
  }
  // ...
}
```

### 2. 优化 `InteractiveTerminal.vue` 生命周期

- 调整 `onMounted` 顺序：**先注册监听器**，再尝试连接 WebSocket
- 这样即使 WebSocket 连接慢，或者失败，缓冲的消息也能立即显示在 xterm 界面上

```typescript
onMounted(async () => {
  initTerminal()
  
  // 立即注册，以便接收缓冲的消息
  unregisterWriteCallback = terminalComposable.onTerminalWrite((content) => {
    terminal.value?.write(content)
  })

  await connect()
})
```

### 3. 优化输出格式

- 在 `useAgentEvents.ts` 中，将换行符 `\n` 转换为 `\r\n`，确保在 xterm.js 中正确换行
- 添加颜色高亮，使命令更醒目

## 预期效果

现在，当 LLM 执行命令时：
1. 如果终端面板未打开，它会自动打开
2. 即使组件正在加载，命令和输出也会被缓冲
3. 一旦组件加载完成，命令和输出会立即显示在终端中
4. 您将看到类似 `$ whoami` 的高亮命令和 `sandbox` 输出

请重新测试。
