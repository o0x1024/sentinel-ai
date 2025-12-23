# 插件编辑器右键菜单优化

## 改进概述

将AI助手的"添加选中"和"添加全部"功能从底部按钮栏移至编辑器右键菜单，提供更符合用户习惯的交互方式。

## 主要变更

### 1. UI调整

#### 删除底部按钮
**位置**：`PluginCodeEditorDialog.vue` - AI Chat Input区域

**移除内容**：
- ❌ "添加选中"按钮
- ❌ "添加全部"按钮  
- ❌ "添加测试结果"按钮（编辑插件时）

**替换为**：
```vue
<div class="flex items-center gap-2 text-xs opacity-60">
  <i class="fas fa-info-circle"></i>
  <span>右键编辑器添加代码到上下文</span>
</div>
```

#### 新增右键菜单
**位置**：`PluginManagement.vue` - Teleport到body

**菜单结构**：
```
┌─────────────────────────────────────┐
│ 🤖 AI 助手                          │
├─────────────────────────────────────┤
│ ⚡ 添加选中代码    Ctrl+Shift+A    │  ← 仅当有选区时显示
│ 📄 添加完整代码    Ctrl+Shift+F    │
└─────────────────────────────────────┘
```

---

## 技术实现

### 1. 右键菜单触发逻辑

```typescript
const setupEditorContextMenu = (editorView: EditorView) => {
  const handleContextMenu = (e: MouseEvent) => {
    // 仅在AI面板开启时显示菜单
    if (!showAiPanel.value) return
    
    // 阻止默认右键菜单
    e.preventDefault()
    e.stopPropagation()
    
    // 检测是否有选区
    const selection = editorView.state.selection.main
    contextMenuHasSelection.value = !selection.empty
    
    // 定位菜单
    contextMenuPosition.value = { x: e.clientX, y: e.clientY }
    showContextMenu.value = true
    
    // 点击其他地方关闭菜单
    const closeMenu = () => {
      showContextMenu.value = false
      document.removeEventListener('click', closeMenu)
      document.removeEventListener('contextmenu', closeMenu)
    }
    
    setTimeout(() => {
      document.addEventListener('click', closeMenu)
      document.addEventListener('contextmenu', closeMenu)
    }, 0)
  }
  
  editorDom.addEventListener('contextmenu', handleContextMenu)
}
```

### 2. 键盘快捷键

```typescript
const handleKeydown = (e: KeyboardEvent) => {
  // Ctrl+Shift+A: 添加选中代码
  if (e.ctrlKey && e.shiftKey && e.key === 'A') {
    e.preventDefault()
    const selection = editorView.state.selection.main
    if (!selection.empty) {
      handleContextMenuAddSelection()
    }
  }
  
  // Ctrl+Shift+F: 添加完整代码
  if (e.ctrlKey && e.shiftKey && e.key === 'F') {
    e.preventDefault()
    handleContextMenuAddAll()
  }
}
```

### 3. 自动打开AI面板

当用户通过右键菜单或快捷键添加代码时，如果AI面板未打开，会自动打开：

```typescript
const handleContextMenuAddSelection = () => {
  showContextMenu.value = false
  addSelectedCodeToContext()
  // 自动打开AI面板
  if (!showAiPanel.value) {
    showAiPanel.value = true
  }
}
```

---

## 样式设计

### 菜单外观

```css
.editor-context-menu {
  position: fixed;
  min-width: 240px;
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.2);
  border-radius: 0.5rem;
  box-shadow: 0 10px 25px -5px rgb(0 0 0 / 0.3);
  z-index: 999999;
  padding: 0.25rem;
  animation: contextMenuFadeIn 0.15s ease-out;
}
```

### 菜单项样式

- **默认状态**：透明背景
- **Hover状态**：`oklch(var(--b3))` 背景
- **图标**：不同颜色区分功能
  - ⚡ 选中代码：`text-warning`（黄色）
  - 📄 完整代码：`text-info`（蓝色）
- **快捷键提示**：右对齐，小号kbd样式

---

## 用户体验改进

### 优点

✅ **符合习惯**：右键菜单是编辑器的标准交互方式  
✅ **减少UI干扰**：移除底部按钮后，聊天区域更简洁  
✅ **上下文感知**：菜单根据选区动态显示选项  
✅ **快捷键支持**：高级用户可以使用键盘快速操作  
✅ **自动引导**：底部提示引导用户使用右键菜单  

### 交互流程

#### 场景1：添加选中代码
```
1. 用户在编辑器中选中代码
2. 右键点击选区
3. 菜单显示"添加选中代码"选项
4. 点击选项
5. AI面板自动打开（如果未开启）
6. 代码引用添加到上下文
```

#### 场景2：添加完整代码
```
1. 用户在编辑器任意位置右键
2. 点击"添加完整代码"
3. AI面板自动打开
4. 完整代码添加到上下文
```

#### 场景3：使用快捷键
```
1. 用户选中代码
2. 按下 Ctrl+Shift+A
3. 直接添加选中代码到上下文
```

---

## 兼容性说明

### 平台快捷键

当前实现使用 `Ctrl` 键，在macOS上应该使用 `Cmd` 键。可以通过以下方式兼容：

```typescript
const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0
const modifierKey = isMac ? e.metaKey : e.ctrlKey

if (modifierKey && e.shiftKey && e.key === 'A') {
  // ...
}
```

**建议**：在后续版本中添加平台检测，提供macOS的 `Cmd` 键支持。

---

## 国际化

### 新增翻译键

需要在国际化文件中添加：

```typescript
// zh.ts
{
  plugins: {
    contextMenuHint: '右键编辑器添加代码到上下文',
    addSelection: '添加选中代码',
    addAll: '添加完整代码',
    aiAssistant: 'AI 助手'
  }
}

// en.ts
{
  plugins: {
    contextMenuHint: 'Right-click editor to add code to context',
    addSelection: 'Add Selected Code',
    addAll: 'Add Full Code',
    aiAssistant: 'AI Assistant'
  }
}
```

---

## 测试建议

### 功能测试

1. **右键菜单显示**
   - ✓ AI面板关闭时，右键不显示菜单
   - ✓ AI面板开启时，右键显示菜单
   - ✓ 有选区时，显示"添加选中"选项
   - ✓ 无选区时，不显示"添加选中"选项

2. **菜单操作**
   - ✓ 点击"添加选中"后，代码引用正确添加
   - ✓ 点击"添加全部"后，完整代码正确添加
   - ✓ 菜单操作后自动关闭
   - ✓ AI面板自动打开

3. **快捷键**
   - ✓ `Ctrl+Shift+A` 添加选中代码
   - ✓ `Ctrl+Shift+F` 添加完整代码
   - ✓ 无选区时，`Ctrl+Shift+A` 不执行

4. **菜单关闭**
   - ✓ 点击菜单外区域关闭
   - ✓ 再次右键其他地方关闭
   - ✓ 按ESC键关闭（待实现）

### 边界测试

- 空编辑器右键
- 极长代码选区
- 快速连续右键
- 全屏模式与普通模式切换
- 主题切换（dark/light）

---

## 后续优化方向

### 1. 菜单功能扩展

可以考虑添加更多上下文相关的功能：

```
┌─────────────────────────────────────┐
│ 🤖 AI 助手                          │
├─────────────────────────────────────┤
│ ⚡ 添加选中代码    Ctrl+Shift+A    │
│ 📄 添加完整代码    Ctrl+Shift+F    │
├─────────────────────────────────────┤
│ 💬 解释这段代码                     │
│ ⚙️ 优化这段代码                     │
│ 🐛 找出潜在问题                     │
└─────────────────────────────────────┘
```

### 2. 智能建议

根据上下文智能显示菜单项：

- 如果选中的是函数，显示"解释函数"、"添加测试用例"
- 如果选中的是注释，显示"生成代码"
- 如果选中的是错误代码，显示"修复错误"

### 3. 子菜单支持

对于复杂功能，支持子菜单：

```
添加到AI上下文 →
  ├─ 作为参考代码
  ├─ 作为修改目标
  └─ 作为示例代码
```

### 4. 历史记录

在菜单中显示最近添加的代码片段，方便快速引用：

```
最近添加 →
  ├─ analyze函数 (30行)
  ├─ 类型定义 (15行)
  └─ 完整插件代码 (200行)
```

---

## 文件修改清单

### 主要文件

1. **`PluginCodeEditorDialog.vue`**
   - 删除底部三个按钮
   - 添加提示文本

2. **`PluginManagement.vue`**
   - 新增右键菜单状态管理
   - 实现 `setupEditorContextMenu()` 函数
   - 添加键盘快捷键处理
   - 添加上下文菜单UI和样式

### 状态变量

```typescript
const showContextMenu = ref(false)
const contextMenuPosition = ref({ x: 0, y: 0 })
const contextMenuHasSelection = ref(false)
```

### 新增函数

```typescript
setupEditorContextMenu()        // 设置右键菜单监听
handleContextMenuAddSelection()  // 处理添加选中代码
handleContextMenuAddAll()        // 处理添加完整代码
```

---

## 总结

通过将功能从按钮栏移至右键菜单，我们实现了：

✅ **更好的用户体验**：符合编辑器使用习惯  
✅ **更简洁的UI**：减少视觉干扰  
✅ **更高的效率**：快捷键+右键双重支持  
✅ **更智能的交互**：上下文感知的菜单  

这个改进为插件编辑器的AI辅助功能提供了更专业和高效的交互方式。

