# 工具执行失败状态显示优化

## 问题
在 ReAct 架构下，工具执行失败的状态没有正确显示出来，用户无法直观看到哪些操作失败了。

## 解决方案

### 1. 增强解析逻辑 (AIChat.vue)

在 `parseReActSteps()` 函数中添加了失败检测逻辑：

#### 检测时机
- 在解析 `Observation:` 行时立即检测
- 收集多行 Observation 内容时持续检测

#### 检测方法
1. **JSON 格式检测**：
   - 尝试解析 Observation 为 JSON
   - 检查 `success: false` 字段
   - 检查 `error` 字段存在
   - 自动设置 action.status = 'failed'

2. **文本关键字检测**：
   - 搜索 "error"、"failed"、"失败" 等关键字
   - 不区分大小写
   - 检测到关键字时设置 action.status = 'failed'

### 2. 视觉状态增强 (ReActStepDisplay.vue)

#### Action 卡片
- **成功状态**：
  - 蓝色背景和边框
  - ▶️ 播放图标
  - 绿色 "已完成" 徽章

- **失败状态**：
  - 红色背景和边框 (bg-error/5, border-error/30)
  - ❌ 错误图标 (fa-times-circle)
  - 红色 "失败" 徽章

#### Observation 卡片
- **成功状态**：
  - 绿色背景和边框
  - ✅ 对勾图标

- **失败状态**：
  - 红色背景和边框 (bg-error/5, border-error/20)
  - ⚠️ 感叹号图标 (fa-exclamation-circle)

### 3. 智能检测函数

#### `hasObservationError(obs)`
检测 Observation 内容是否包含错误：

```typescript
const hasObservationError = (obs: any) => {
  if (typeof obs === 'string') {
    const lowerObs = obs.toLowerCase()
    return lowerObs.includes('error') || 
           lowerObs.includes('failed') || 
           lowerObs.includes('失败') ||
           lowerObs.includes('"success":false') ||
           lowerObs.includes('"success": false')
  }
  if (typeof obs === 'object' && obs !== null) {
    return obs.success === false || obs.error
  }
  return false
}
```

#### `getActionStatusText(status)`
将状态码转换为中文文本：
- `running` → 运行中
- `success` → 成功
- `completed` → 已完成
- `failed` → 失败
- `error` → 错误

## 效果展示

### 失败状态示例

```
Thought: 搜索框选择器可能不正确

Action: playwright_fill
Action Input: {"selector":"input[name='wd']","value":"今日热点"}

Observation: {"success":false,"error":"Selector not found: input[name='wd']"}
```

**显示效果**：
- 🔴 红色背景的 Action 卡片
- ❌ 错误图标
- 🔴 红色 "失败" 徽章
- ⚠️ 红色背景的 Observation 卡片
- 清晰显示错误信息

### 成功状态示例

```
Action: playwright_get_visible_text
Action Input: {}

Observation: {"success":true,"output":"..."}
```

**显示效果**：
- 🔵 蓝色背景的 Action 卡片
- ▶️ 播放图标
- 🟢 绿色 "已完成" 徽章
- ✅ 绿色背景的 Observation 卡片

## 技术细节

### 颜色方案
- **成功**: 绿色系 (success/green)
- **失败**: 红色系 (error/red)
- **运行中**: 橙色系 (warning/orange)
- **常规**: 蓝色系 (primary/blue)

### 响应式检测
- 支持 JSON 格式的错误响应
- 支持纯文本错误消息
- 支持中英文错误关键字
- 自动适配不同的错误格式

### 用户体验
- 失败状态一目了然
- 颜色编码清晰
- 图标语义明确
- 悬停效果增强交互

## 测试建议

1. **成功场景**：执行成功的 playwright 工具调用
2. **JSON 错误**：返回 `{"success": false}` 的场景
3. **文本错误**：返回包含 "error" 关键字的场景
4. **中文错误**：返回包含 "失败" 的场景
5. **混合内容**：同时有成功和失败的多步骤场景

## 文件变更

- ✅ `/src/components/AIChat.vue` - 添加失败检测逻辑
- ✅ `/src/components/MessageParts/ReActStepDisplay.vue` - 增强视觉状态
- ✅ `/docs/react_display_preview.html` - 添加失败示例

## 预览

打开 `/docs/react_display_preview.html` 可查看完整的成功和失败状态展示效果。
