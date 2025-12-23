# Plugin Code Editor与AI助手联动改进

## 改进概览

本次改进针对插件编辑器与AI助手之间的联动体验进行了全面优化，解决了多个关键问题，提升了用户体验和功能完整性。

## 核心改进

### 1. 编辑器状态保存与恢复 ✅

**问题描述**：
在普通编辑模式和全屏编辑模式之间切换时，编辑器实例会被销毁并重建，导致以下状态丢失：
- 光标位置
- 滚动条位置
- 文本选区
- 编辑历史（Undo/Redo栈）

**解决方案**：
```typescript
interface EditorViewState {
  cursorPos: number
  scrollTop: number
  scrollLeft: number
  selectionRanges: Array<{ from: number; to: number }>
}
```

实现了 `saveEditorState()` 和 `restoreEditorState()` 函数：
- 在进入全屏模式前保存普通编辑器状态
- 在全屏编辑器初始化后恢复状态
- 在退出全屏模式时保存全屏编辑器状态并恢复到普通编辑器

**效果**：
- 用户可以在任意位置切换全屏模式，无需重新定位
- 保持编辑上下文连续性
- 提升大文件编辑体验

---

### 2. Diff预览功能实现 ✅

**问题描述**：
`previewAiCode` 功能仅输出日志，没有实际的代码对比视图，用户无法直观看到AI修改建议与当前代码的差异。

**解决方案**：

1. **UI层**：添加了diff视图容器和header
```vue
<div v-show="isPreviewMode" ref="fullscreenDiffEditorContainerRef">
  <div class="diff-header">
    <!-- 对比信息和操作按钮 -->
  </div>
</div>
```

2. **逻辑层**：实现了并排对比视图
- 左侧：当前代码（只读）
- 右侧：AI修改后代码（可编辑）
- 支持在预览模式下进一步调整代码
- 提供"应用修改"和"退出预览"操作

3. **交互流程**：
```
AI返回代码 → 点击"预览" → 进入Diff模式 → 
查看/编辑对比 → 点击"应用" → 更新主编辑器 → 退出预览
```

**效果**：
- 用户可以清晰看到AI的修改建议
- 支持在应用前进行微调
- 降低误操作风险

---

### 3. AI面板拖拽调整宽度 ✅

**问题描述**：
AI面板宽度固定（400px），在不同屏幕或查看长代码时体验不佳。

**解决方案**：

1. 添加了可视化的resize handle：
```css
.resize-handle {
  position: absolute;
  left: 0;
  width: 6px;
  cursor: col-resize;
  /* 视觉反馈效果 */
}
```

2. 实现了拖拽逻辑：
- `startResize()`: 监听鼠标按下
- `handleResize()`: 动态计算宽度（300px-800px）
- `stopResize()`: 释放监听器

3. 宽度约束：
- 最小宽度：300px（保证可读性）
- 最大宽度：800px（避免遮挡过多编辑区）

**效果**：
- 用户可以根据需要调整AI面板大小
- 提升多任务并行体验
- 适配不同屏幕尺寸

---

### 4. 智能代码应用逻辑 ✅

**问题描述**：
AI返回的代码直接全量替换，无法区分局部修改、完整替换或代码追加。

**解决方案**：

实现了 `detectCodeApplicationMode()` 智能识别：

```typescript
function detectCodeApplicationMode(aiCode, currentCode): 'full' | 'partial' | 'append' {
  // 1. 检测完整插件结构标记
  const markers = ['export interface', 'export async function analyze', ...]
  
  // 2. 计算代码行数比例
  const ratio = aiLines / currentLines
  
  // 3. 识别注释追加模式
  if (aiCode.startsWith('//') || aiCode.startsWith('/*')) return 'append'
  
  // 4. 综合判断
  if (hasMarkers && ratio > 0.5) return 'full'
  if (ratio < 0.3) return 'partial'
}
```

**应用策略**：
- **全量替换**：AI返回完整插件代码
- **片段替换**：提示用户（未来可扩展为智能定位）
- **追加模式**：追加到文件末尾

**效果**：
- 提供用户明确的操作反馈
- 降低代码丢失风险
- 为未来的智能合并奠定基础

---

### 5. Markdown渲染优化 ✅

**问题描述**：
流式输出的Markdown格式在未闭合时导致布局跳动，代码块高亮失效。

**解决方案**：

实现了增强的 `renderMarkdown()` 函数：

```typescript
function renderMarkdown(content: string): { html: string; codeBlock?: string } {
  // 1. 提取并保护代码块（避免被其他规则破坏）
  const codeBlocks: string[] = []
  html = html.replace(/```.*?\n?([\s\S]*?)```/g, (match, code) => {
    codeBlocks.push(code.trim())
    return `___CODE_BLOCK_${index}___`
  })
  
  // 2. 处理内联代码
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>')
  
  // 3. 处理格式（加粗、列表等）
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
  
  // 4. 恢复代码块（使用安全的HTML转义）
  codeBlocks.forEach((code, i) => {
    html = html.replace(`___CODE_BLOCK_${i}___`, 
      `<pre><code>${escapeHtml(code)}</code></pre>`)
  })
}
```

**效果**：
- 流式输出时布局稳定
- 代码块正确高亮
- 防止XSS注入（通过HTML转义）

---

### 6. 代码引用实时更新 ✅

**问题描述**：
用户"添加代码到上下文"后继续编辑，发送消息时AI收到的仍是旧代码。

**解决方案**：

1. **实时获取代码**：
```typescript
const getCurrentEditorCode = (): string => {
  const editorView = isFullscreenEditor.value 
    ? fullscreenCodeEditorView 
    : codeEditorView
  return editorView?.state.doc.toString() || pluginCode.value
}
```

2. **发送前刷新引用**：
```typescript
const sendAiChatMessage = async (message: string) => {
  // 刷新代码引用
  if (codeRef) {
    const currentCode = getCurrentEditorCode()
    if (codeRef.isFullCode) {
      codeRef.code = currentCode
    } else {
      // 按行号范围刷新选区
      const lines = currentCode.split('\n')
      codeRef.code = lines.slice(startIdx, endIdx).join('\n')
    }
  }
  // ... 发送消息
}
```

**效果**：
- AI始终基于最新代码工作
- 避免"时间差"导致的错误建议
- 提升编辑-咨询循环的效率

---

## 技术亮点

### 状态管理
- 引入了 `EditorViewState` 接口统一管理编辑器状态
- 使用 `nextTick()` 和 `setTimeout()` 确保DOM更新后再操作
- 分离了"数据状态"和"视图状态"

### 性能优化
- Diff视图采用按需创建、用后销毁策略
- 避免内存泄漏（及时清理监听器和编辑器实例）
- 使用CSS变量适配DaisyUI 4.x主题系统

### 用户体验
- 所有操作提供明确的Toast反馈
- 支持ESC键快速退出全屏/预览模式
- Resize handle有视觉反馈（hover高亮）

---

## 后续可优化方向

### 1. 真正的Diff算法集成
当前Diff视图是简单的并排对比，可引入：
- 第三方库如 `diff-match-patch`
- 行级别的增删改标记
- 代码块折叠/展开

### 2. 智能代码合并
当前"partial"模式只是提示，可实现：
- 基于AST的智能定位
- 函数级别的精确替换
- 冲突检测与解决

### 3. 编辑历史持久化
当前Undo栈在切换模式时丢失，可考虑：
- 使用 `@codemirror/commands` 的 `history` 扩展
- 保存历史记录到内存/IndexedDB
- 跨模式的统一历史栈

### 4. AI上下文增强
- 支持多轮对话的上下文累积
- 引用历史测试结果
- 自动附加相关文档

---

## 文件修改清单

### 主要文件
1. `/src/views/PluginManagement.vue`
   - 新增编辑器状态管理
   - 实现Diff预览逻辑
   - 优化AI消息处理

2. `/src/components/PluginManagement/PluginCodeEditorDialog.vue`
   - 添加Diff视图容器
   - 完善resize handle样式
   - 暴露必要的ref

### 类型定义
- `EditorViewState`: 编辑器状态接口
- `isPreviewMode`: 预览模式标志
- `previewCode`: 预览代码缓存

---

## 测试建议

### 功能测试
1. **状态保存**
   - 在代码中定位到某行
   - 切换全屏模式
   - 验证光标位置不变

2. **Diff预览**
   - 与AI对话获取代码修改
   - 点击"预览"按钮
   - 检查左右对比视图
   - 编辑右侧代码
   - 应用修改

3. **面板调整**
   - 拖拽AI面板左边缘
   - 验证宽度动态变化
   - 测试最小/最大宽度限制

4. **智能应用**
   - 测试完整插件代码替换
   - 测试代码片段追加
   - 验证Toast提示内容

5. **实时引用**
   - 添加代码到上下文
   - 继续编辑代码
   - 发送AI消息
   - 验证AI回复基于最新代码

### 边界测试
- 空代码处理
- 超长代码（>10000行）
- 频繁切换模式
- 网络中断时的流式输出

---

## 总结

通过本次改进，插件编辑器与AI助手的联动体验得到了质的提升：

✅ **稳定性增强**：状态保存避免了用户操作丢失  
✅ **功能完整**：Diff预览从"占位符"变为可用功能  
✅ **灵活性提升**：可调整UI布局适配不同场景  
✅ **智能化提升**：代码应用更加智能和安全  
✅ **实时性保障**：消除代码引用的时间差问题  

这些改进为后续的协作式AI编程功能奠定了坚实基础。


