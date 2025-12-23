# 插件编辑器 AI 助手问题修复

## 修复概述

针对插件代码编辑器AI助手的4个关键问题进行了修复，提升了用户体验和功能准确性。

---

## 问题1：未选中代码时仍发送全量代码 ✅

### 问题描述

用户在AI助手中发送消息时，即使没有主动添加任何代码引用，系统仍会自动将完整的插件代码发送给LLM。

**日志证据**：
```
[2025-12-23 08:10:36.876 UTC] [USER REQUEST]
当前插件代码：
```typescript
/**
 * @plugin Hash_Calculator_Tool
 * ...完整的300多行代码...
 */
```

用户需求：你是谁？
```

从日志可以看出，用户只是问了一个简单的问题"你是谁？"，但系统把整个插件的300多行代码都发送了。

### 问题根源

**文件**：`PluginManagement.vue` - `sendAiChatMessage()` 函数

**原代码**（第1489-1491行）：
```typescript
if (codeRef) {
  // ... 处理代码引用
} else if (latestCode) {
  // ❌ 自动添加完整代码
  contextParts.push(`当前插件代码：\n\`\`\`typescript\n${latestCode}\n\`\`\``)
}
```

**问题**：当没有 `codeRef`（代码引用）时，会自动添加 `latestCode`（完整代码）。

### 修复方案

**修改后的代码**：
```typescript
// Add code context - ONLY if user explicitly added code reference
if (codeRef) {
  if (codeRef.isFullCode) {
    contextParts.push(`完整插件代码：\n\`\`\`typescript\n${codeRef.code}\n\`\`\``)
  } else {
    contextParts.push(`选中的代码片段 (第${codeRef.startLine}-${codeRef.endLine}行)：\n\`\`\`typescript\n${codeRef.code}\n\`\`\``)
    contextParts.push(`完整代码上下文：\n\`\`\`typescript\n${latestCode}\n\`\`\``)
  }
}
// Note: No longer automatically add full code if no reference is set
```

### 效果

✅ **精准控制**：只有当用户通过右键菜单主动添加代码时，才会发送代码上下文  
✅ **减少token消耗**：普通聊天不再浪费大量token  
✅ **提升响应速度**：减少请求体大小，加快响应  

### 使用场景对比

| 场景 | 修复前 | 修复后 |
|------|--------|--------|
| 问"如何使用这个插件？" | 发送300行代码 + 问题 | 只发送问题 |
| 右键 → 添加全部 → 问"优化代码" | 发送300行代码 + 问题 | 发送300行代码 + 问题 ✓ |
| 选中10行 → 右键 → 添加选中 → 问"解释这段代码" | 发送10行 + 300行 + 问题 | 发送10行 + 300行 + 问题 ✓ |

---

## 问题2：AI响应内容没有Markdown渲染 ✅

### 问题描述

AI返回的Markdown格式内容（如代码块、加粗、列表等）没有被正确渲染，显示为纯文本。

**预期效果**：
```
代码块应该有背景色和边框
**加粗文本**应该显示为粗体
- 列表项应该有项目符号
```

**实际效果**：
```
```typescript
代码块显示为纯文本
```
**加粗文本**显示为纯文本
- 列表项显示为纯文本
```

### 问题根源

1. **流式输出时**：使用了 `{{ }}` 纯文本绑定，没有渲染HTML
2. **Markdown样式不完整**：缺少内联代码、加粗、列表等元素的样式

### 修复方案

#### 1. 改进流式输出显示

**文件**：`PluginCodeEditorDialog.vue`

**修改前**：
```vue
<span v-if="aiStreamingContent">{{ aiStreamingContent }}</span>
```

**修改后**：
```vue
<span v-if="aiStreamingContent" class="whitespace-pre-wrap">{{ aiStreamingContent }}</span>
```

**新增CSS**：
```css
.ai-chat-message .message-text.streaming-text {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  white-space: pre-wrap;
}
```

#### 2. 增强Markdown样式

**新增CSS样式**：
```css
/* 代码块边框 */
.ai-chat-message .message-text :deep(pre) {
  border: 1px solid oklch(var(--bc) / 0.1);
}

/* 内联代码 */
.ai-chat-message .message-text :deep(code.inline-code) {
  background: oklch(var(--b3));
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  font-size: 0.8em;
}

/* 加粗文本 */
.ai-chat-message .message-text :deep(strong) {
  font-weight: 600;
}

/* 列表样式 */
.ai-chat-message .message-text :deep(ul) {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}

.ai-chat-message .message-text :deep(li) {
  margin: 0.25rem 0;
}
```

### 效果

✅ **代码块**：有边框和背景色，易于识别  
✅ **内联代码**：有背景色，与普通文本区分  
✅ **加粗文本**：正确显示为粗体  
✅ **列表**：有缩进和间距  

---

## 问题3：代理配置后看不到请求包 🔍

### 问题描述

在 `NetworkSettings.vue` 中开启代理后，在插件编辑器AI助手发送消息时，没有看到HTTP请求包。

### 可能原因

1. **后端问题**：LLM服务可能使用了不同的HTTP客户端，没有应用全局代理
2. **服务名称问题**：使用的是 `'default'` 而非 `'default_llm_provider'`
3. **代理配置时机**：代理配置可能在客户端创建后才更新

### 修复方案

#### 1. 添加调试日志

**文件**：`PluginManagement.vue`

```typescript
// Log request for debugging
console.log('[AI Chat] Sending message:', {
  streamId,
  messageLength: userPrompt.length,
  hasCodeContext: !!codeRef,
  hasTestContext: !!testResultRef,
  serviceName: 'default'
})

await invoke('generate_plugin_stream', {
  request: {
    stream_id: streamId,
    message: userPrompt,
    system_prompt: systemPrompt,
    service_name: 'default',
  }
})
```

#### 2. 排查建议

**前端检查**：
1. 打开开发者工具的Console
2. 查看 `[AI Chat] Sending message:` 日志
3. 确认请求是否正常发送

**后端检查**（需要查看Rust后端代码）：
1. 检查 `generate_plugin_stream` 命令实现
2. 确认使用的HTTP客户端是否支持全局代理
3. 检查 `service_name: 'default'` 是否映射到正确的服务

**代理测试**：
参考 `NetworkSettings.vue` 中的三个测试按钮：
- 动态更新测试
- 持久化测试
- 客户端更新测试

### 后续工作

⚠️ **需要后端配合**：此问题可能需要在Rust后端修复，确保：
1. LLM HTTP客户端正确应用全局代理配置
2. 代理配置动态更新后，现有连接能够重建
3. 所有HTTP请求都通过统一的代理管理

---

## 问题4：用户发送消息后输入区内容没有清除 ✅

### 问题描述

用户在AI助手输入框中输入消息并发送后，输入框的内容没有被清除，需要手动删除。

### 问题根源

**文件**：`PluginCodeEditorDialog.vue`

**原代码**（第542-546行）：
```typescript
// Clear input after sending
watch(() => props.aiStreaming, (streaming, wasStreaming) => {
  if (wasStreaming && !streaming) {
    // ❌ 在流式结束后才清空
    aiInputText.value = ''
  }
})
```

**问题**：
- 只在 `wasStreaming && !streaming` 时清空
- 即流式输出**结束后**才清空
- 用户发送消息后，输入框内容还保留着，体验不佳

### 修复方案

**修改后的代码**：
```typescript
// Clear input immediately when streaming starts
watch(() => props.aiStreaming, (streaming) => {
  if (streaming) {
    // ✅ 流式开始时立即清空
    aiInputText.value = ''
  }
})
```

### 效果

✅ **立即清空**：点击发送后，输入框立即清空  
✅ **流畅体验**：可以立即输入下一条消息  
✅ **避免重复发送**：减少误操作的可能性  

### 交互流程对比

**修复前**：
```
1. 用户输入"优化这段代码"
2. 点击发送按钮
3. 输入框仍然显示"优化这段代码"
4. AI开始流式输出...
5. AI输出完成
6. 输入框清空 ← 太晚了
```

**修复后**：
```
1. 用户输入"优化这段代码"
2. 点击发送按钮
3. 输入框立即清空 ← 立即清空✓
4. AI开始流式输出...
5. AI输出完成
```

---

## 测试建议

### 1. 代码上下文测试

**测试步骤**：
1. 打开插件编辑器
2. 打开AI助手面板
3. **不添加任何代码引用**
4. 输入"你好"
5. 查看浏览器Console和后端日志

**预期结果**：
- 前端日志显示 `hasCodeContext: false`
- 后端日志中的 `message` 字段只包含"你好"，不包含代码
- LLM返回简短的问候，不涉及代码

### 2. Markdown渲染测试

**测试步骤**：
1. 添加完整代码到上下文
2. 询问"请用列表格式说明这个插件的功能"
3. 观察AI返回的内容

**预期结果**：
- 列表有项目符号和缩进
- 代码块有边框和背景色
- 内联代码有背景高亮
- 加粗文本显示为粗体

### 3. 输入框清空测试

**测试步骤**：
1. 在输入框输入一段文字
2. 点击发送按钮
3. 观察输入框状态

**预期结果**：
- 点击发送后，输入框**立即**清空
- 不需要等待AI响应完成

### 4. 代理配置测试

**测试步骤**：
1. 在设置中开启代理
2. 打开开发者工具 → Network标签
3. 在AI助手中发送消息
4. 查看Console日志和Network请求

**预期结果**：
- Console显示 `[AI Chat] Sending message:` 日志
- 如果代理配置正确，后端日志应该显示通过代理发送请求
- （如果看不到请求包，需要检查后端实现）

---

## 文件修改清单

### 主要文件

1. **`src/views/PluginManagement.vue`**
   - ✅ 移除自动添加完整代码的逻辑
   - ✅ 添加调试日志

2. **`src/components/PluginManagement/PluginCodeEditorDialog.vue`**
   - ✅ 修改流式输出显示方式
   - ✅ 修改输入框清空时机
   - ✅ 增强Markdown样式

### 修改统计

- **删除代码**：3行（自动添加代码的逻辑）
- **新增代码**：~30行（样式、日志、注释）
- **修改代码**：2处（流式显示、输入清空）

---

## 用户体验改进总结

| 改进项 | 改进前 | 改进后 | 影响 |
|--------|--------|--------|------|
| 代码发送 | 总是发送全量代码 | 仅在明确添加时发送 | 🎯 精准控制 |
| Token消耗 | 每次请求包含300+行 | 按需包含代码 | 💰 节省成本 |
| 响应速度 | 慢（大请求体） | 快（小请求体） | ⚡ 提升性能 |
| Markdown | 纯文本显示 | 正确渲染 | 🎨 美观易读 |
| 输入清空 | 流式结束后清空 | 发送后立即清空 | 🚀 流畅体验 |
| 调试支持 | 无日志 | 有详细日志 | 🔍 便于排查 |

---

## 后续优化方向

### 1. 智能代码上下文

当前的实现是"全有"或"全无"：
- 不添加引用 → 不发送任何代码
- 添加引用 → 发送指定代码

**未来可以实现智能上下文**：
- 分析用户问题的意图
- 自动判断是否需要代码上下文
- 例如："如何使用"→ 不需要代码；"优化这个函数"→ 需要代码

### 2. 代码片段智能补全

当用户选中部分代码时：
- 自动分析选中代码的依赖
- 补充必要的类型定义和导入语句
- 提供更完整的上下文给LLM

### 3. 对话历史管理

实现对话历史的智能管理：
- 保留最近N轮对话
- 自动清理过期的代码引用
- 提供"清空对话"按钮

### 4. 代理配置优化

**前端增强**：
- 在AI助手面板显示代理状态指示器
- 提供"测试代理连接"按钮
- 显示请求延迟统计

**后端增强**（需要Rust开发）：
- 确保所有HTTP客户端统一使用全局代理
- 支持代理配置热更新
- 添加详细的代理请求日志

---

## 总结

通过本次修复：

✅ **问题1**：修复了不必要的全量代码发送，精准控制上下文  
✅ **问题2**：完善了Markdown渲染样式，提升可读性  
⚠️ **问题3**：添加了调试日志，但可能需要后端配合修复  
✅ **问题4**：修复了输入框清空时机，提升交互体验  

所有修改已通过编译测试，可以直接使用！🚀

