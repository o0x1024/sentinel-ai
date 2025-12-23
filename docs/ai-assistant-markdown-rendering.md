# AI 助手 Markdown 渲染升级

## 概述

AI 助手的响应消息现在使用 `marked` 库进行完整的 Markdown 渲染，提供更好的格式化支持。

## 技术实现

### 使用的库

- **marked** (v17.0.0): 高性能 Markdown 解析器
- **DOMPurify** (v3.2.6): HTML 清理库，防止 XSS 攻击

### 配置选项

```javascript
marked.setOptions({
  breaks: true,    // 支持换行符转换为 <br>
  gfm: true,       // 支持 GitHub Flavored Markdown
})
```

### 安全性

使用 DOMPurify 清理所有 HTML 输出，只允许安全的标签：

```javascript
DOMPurify.sanitize(rawHtml, {
  ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'code', 'pre', 'ul', 'ol', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'a'],
  ALLOWED_ATTR: ['href', 'class']
})
```

## 支持的 Markdown 特性

### 1. 标题
```markdown
# 一级标题
## 二级标题
### 三级标题
```

### 2. 文本格式
```markdown
**粗体文本**
*斜体文本*
`内联代码`
```

### 3. 代码块
````markdown
```typescript
export function example() {
  return "Hello World"
}
```
````

### 4. 列表
```markdown
- 无序列表项 1
- 无序列表项 2

1. 有序列表项 1
2. 有序列表项 2
```

### 5. 引用
```markdown
> 这是一段引用文本
```

### 6. 链接
```markdown
[链接文本](https://example.com)
```

## 实时渲染

### 流式输出

AI 消息在流式输出时也会实时渲染 Markdown：

```vue
const aiStreamingContentRendered = computed(() => {
  if (!props.aiStreamingContent) return ''
  const rawHtml = marked.parse(props.aiStreamingContent) as string
  return DOMPurify.sanitize(rawHtml, { ... })
})
```

### 完整消息

完整消息在 `finishAiChat` 函数中渲染：

```typescript
const finishAiChat = (content: string) => {
  const { html, codeBlocks } = renderMarkdown(content)
  
  aiChatMessages.value.push({ 
    role: 'assistant', 
    content: html,
    codeBlock: codeBlocks[0],
    codeBlocks: codeBlocks
  })
}
```

## 代码块提取

为了支持"应用代码"功能，系统会在渲染前提取所有代码块：

```typescript
const codeBlockRegex = /```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/g
let match
while ((match = codeBlockRegex.exec(content)) !== null) {
  codeBlocks.push(match[1].trim())
}
```

## 样式定制

### 通用元素样式

```css
.ai-chat-message .message-text :deep(p) {
  margin-bottom: 0.75rem;
}

.ai-chat-message .message-text :deep(h1) { 
  font-size: 1.25rem; 
  font-weight: bold;
}

.ai-chat-message .message-text :deep(blockquote) {
  border-left: 3px solid oklch(var(--p) / 0.3);
  padding-left: 1rem;
  margin: 0.75rem 0;
}
```

### 代码块样式

```css
.ai-chat-message .message-text :deep(pre) {
  background: oklch(var(--b1));
  padding: 0.75rem;
  border-radius: 0.5rem;
  border: 1px solid oklch(var(--bc) / 0.1);
}

.ai-chat-message .message-text :deep(code.inline-code) {
  background: oklch(var(--b3));
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
}
```

## 优势

### 与手动解析相比

| 特性 | 手动解析 | marked 库 |
|------|---------|-----------|
| 复杂 Markdown | ❌ 不支持 | ✅ 完整支持 |
| 嵌套列表 | ❌ 有限支持 | ✅ 完整支持 |
| 表格 | ❌ 不支持 | ✅ 支持 GFM 表格 |
| 链接 | ❌ 需要手动实现 | ✅ 原生支持 |
| 转义字符 | ❌ 容易出错 | ✅ 自动处理 |
| 维护成本 | ⚠️ 高 | ✅ 低 |

### 性能

- `marked` 是一个高性能解析器，对于典型的 AI 响应（几百到几千字符）处理时间小于 1ms
- DOMPurify 的清理过程也非常快速

## 测试建议

### 1. 基础格式测试

在 AI 助手中输入：
```
请用 Markdown 格式回答：
- 使用**粗体**和*斜体*
- 添加代码示例
- 使用列表和标题
```

### 2. 代码块测试

```
请给我一个 TypeScript 函数示例，并解释它的功能
```

预期 AI 会返回：
````
这是一个示例函数：

```typescript
function greet(name: string): string {
  return `Hello, ${name}!`
}
```

这个函数接受一个字符串参数...
````

### 3. 复杂 Markdown 测试

```
请用 Markdown 格式总结以下内容：
1. 主要功能
2. 使用方法（包含代码示例）
3. 注意事项（使用引用块）
```

## 故障排查

### 问题：代码块未正确提取

**原因**：代码块正则表达式可能无法匹配某些格式。

**解决**：检查 `codeBlockRegex` 是否覆盖了所有语言标识符：
```javascript
/```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/g
```

### 问题：HTML 被过度清理

**原因**：DOMPurify 配置过于严格。

**解决**：在 `ALLOWED_TAGS` 中添加需要的标签：
```javascript
ALLOWED_TAGS: [..., 'table', 'thead', 'tbody', 'tr', 'td', 'th']
```

### 问题：流式输出时 Markdown 渲染闪烁

**原因**：每次增量都重新解析整个内容。

**解决**：这是正常行为，`marked` 需要完整上下文才能正确解析。可以考虑添加防抖。

## 未来增强

1. **语法高亮**：集成 `highlight.js` 为代码块添加语法高亮
2. **数学公式**：支持 LaTeX 数学公式渲染
3. **流程图**：支持 Mermaid 图表渲染
4. **表格**：优化表格样式
5. **Emoji**：支持 Emoji 快捷码

## 相关文件

- `src/views/PluginManagement.vue`: 主要的 Markdown 渲染逻辑
- `src/components/PluginManagement/PluginCodeEditorDialog.vue`: 流式渲染和样式
- `src/components/PluginManagement/types.ts`: 消息类型定义

## 总结

通过集成 `marked` 和 `DOMPurify`，AI 助手现在可以：

✅ 渲染完整的 Markdown 格式  
✅ 安全地显示 AI 生成的内容  
✅ 支持流式输出的实时渲染  
✅ 提取代码块用于"应用修改"功能  
✅ 提供更好的用户体验  

这使得插件编辑器的 AI 助手更接近专业的 AI 编程工具！🚀

