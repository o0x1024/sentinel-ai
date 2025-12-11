# 知识库引用检测功能 - 实现总结

## 功能概述

实现了完整的知识库引用检测和可视化功能，用户可以清楚地知道AI是否真正使用了知识库中的内容。

## 实现的功能

### 1. 后端检测机制 ✅

**文件**: `src-tauri/src/commands/ai.rs`

- ✅ RAG启用时注入知识溯源规范到系统提示
- ✅ 要求AI使用 `[SOURCE n]` 格式引用证据
- ✅ 发送 `ai_meta_info` 事件通知前端RAG已应用
- ✅ 支持超时控制的知识库检索（1.2秒）
- ✅ 自动合并角色提示和系统提示

### 2. 前端检测逻辑 ✅

**文件**: `src/composables/useAgentEvents.ts`

- ✅ 监听 `ai_meta_info` 事件
- ✅ 使用正则表达式检测 `[SOURCE n]` 引用
- ✅ 统计引用次数
- ✅ 区分"RAG已启用"和"真正引用了知识库"
- ✅ 将RAG信息附加到消息元数据

**新增类型定义**:
```typescript
interface RagMetaInfo {
  rag_applied: boolean        // RAG是否已应用
  rag_sources_used: boolean   // 是否实际引用了知识库
  source_count: number        // 引用次数
}
```

### 3. UI可视化 ✅

#### 3.1 会话头部指示器
**文件**: `src/components/Agent/AgentView.vue`

- ✅ 显示 📖 知识库 标签（RAG启用时）
- ✅ 蓝色背景和边框
- ✅ 位置：会话标题右侧

#### 3.2 消息级别指示器
**文件**: `src/components/Agent/MessageBlock.vue`

- ✅ 显示引用状态横幅
- ✅ 两种状态：
  - "已引用知识库 (n 处引用)"
  - "已启用知识库，但未找到相关内容"
- ✅ 蓝色信息样式

#### 3.3 引用高亮显示
**文件**: `src/components/Agent/MarkdownRenderer.vue`

- ✅ 自动识别并高亮 `[SOURCE n]` 标记
- ✅ 蓝色背景和边框
- ✅ 等宽字体显示
- ✅ 悬停提示："知识库引用 #n"
- ✅ 悬停动画效果（上移+阴影）

#### 3.4 输入框提示
**文件**: `src/components/InputAreaComponent.vue`

- ✅ RAG按钮的tooltip更新
- ✅ 说明AI将使用 `[SOURCE n]` 格式引用

## 代码变更清单

### 修改的文件

1. ✅ `src/composables/useAgentEvents.ts`
   - 添加 `RagMetaInfo` 类型
   - 添加 `ragMetaInfo` 响应式变量
   - 监听 `ai_meta_info` 事件
   - 在 `assistant_message_saved` 和 `complete` 事件中检测引用
   - 导出 `ragMetaInfo` 到返回值

2. ✅ `src/components/Agent/MessageBlock.vue`
   - 添加 RAG 引用指示器组件
   - 计算 `ragInfo` 属性
   - 显示引用状态

3. ✅ `src/components/Agent/MarkdownRenderer.vue`
   - 添加 `[SOURCE n]` 正则替换逻辑
   - 添加 `.source-citation` 样式
   - 实现悬停效果

4. ✅ `src/components/Agent/AgentView.vue`
   - 在会话头部添加知识库状态标签

5. ✅ `src/components/InputAreaComponent.vue`
   - 更新RAG按钮的tooltip文本

### 新增的文件

1. ✅ `docs/rag-citation-detection.md` - 技术实现文档
2. ✅ `docs/rag-usage-example.md` - 使用示例和最佳实践
3. ✅ `docs/rag-citation-summary.md` - 本文档

## 技术细节

### 引用检测正则表达式
```typescript
const sourcePattern = /\[SOURCE\s+\d+\]/gi
```
- 匹配格式：`[SOURCE 1]`, `[SOURCE 2]`, 等
- 不区分大小写
- 允许SOURCE和数字之间有空格

### 数据流

```
用户开启RAG
    ↓
后端检索知识库
    ↓
注入系统提示（要求使用[SOURCE n]）
    ↓
发送 ai_meta_info 事件
    ↓
前端标记 rag_applied=true
    ↓
AI流式响应
    ↓
前端检测 [SOURCE n] 模式
    ↓
更新 rag_sources_used 和 source_count
    ↓
UI显示引用状态和高亮
```

### 样式规范

**知识库标签** (会话头部):
```css
background: hsl(var(--in) / 0.1)
border: 1px solid hsl(var(--in) / 0.3)
color: hsl(var(--in))
```

**引用指示器** (消息级别):
```css
background: hsl(var(--in) / 0.1)
border: 1px solid hsl(var(--in) / 0.3)
```

**引用标记** (文本内):
```css
background: hsl(var(--in) / 0.15)
border: 1px solid hsl(var(--in) / 0.3)
font-family: monospace
```

## 测试验证

### 编译测试 ✅

```bash
# 前端编译
npm run build
# ✅ 成功，无错误

# 后端编译
cd src-tauri && cargo check
# ✅ 成功，仅有警告（未使用的函数）
```

### 功能测试场景

#### 场景1: 成功引用知识库
1. 开启RAG功能
2. 提问："什么是XSS攻击？"
3. 预期结果：
   - ✅ 会话头部显示 📖 知识库
   - ✅ AI回复包含 `[SOURCE n]` 标记
   - ✅ 标记被高亮显示
   - ✅ 消息下方显示"已引用知识库 (n 处引用)"

#### 场景2: 无相关内容
1. 开启RAG功能
2. 提问："今天天气怎么样？"
3. 预期结果：
   - ✅ 会话头部显示 📖 知识库
   - ✅ AI回复不包含 `[SOURCE n]`
   - ✅ 消息下方显示"已启用知识库，但未找到相关内容"

#### 场景3: 未开启RAG
1. 不开启RAG功能
2. 正常对话
3. 预期结果：
   - ✅ 不显示任何知识库相关指示器
   - ✅ 正常对话流程

## 用户体验改进

### 透明性
- ✅ 用户可以清楚看到RAG是否启用
- ✅ 用户可以知道AI是否真的使用了知识库
- ✅ 用户可以看到引用了多少次

### 可追溯性
- ✅ 每个引用都有明确的标记 `[SOURCE n]`
- ✅ 引用标记视觉上突出显示
- ✅ 悬停时显示提示信息

### 可靠性
- ✅ 区分"知识库已启用"和"真正引用了知识库"
- ✅ 准确统计引用次数
- ✅ 实时检测和更新状态

## 未来改进方向

### 短期（已规划）
1. 引用详情弹窗 - 点击 `[SOURCE n]` 显示原始文档片段
2. 引用列表侧边栏 - 显示所有引用的来源
3. 引用质量评分 - 评估引用的相关性

### 中期
1. 引用管理 - 收藏、标注、分享引用
2. 引用统计 - 知识库使用频率分析
3. 多知识库支持 - 同时检索多个集合

### 长期
1. 智能引用推荐 - 主动推荐相关知识
2. 引用网络图 - 可视化知识关联
3. 协同标注 - 团队共同改进知识库

## 相关文档

- [技术实现详解](./rag-citation-detection.md)
- [使用示例和最佳实践](./rag-usage-example.md)
- [RAG配置指南](./rag-configuration.md) (如果存在)

## 总结

本次实现完成了知识库引用检测的完整功能链路：

1. **后端**: 注入引用规范 → 发送元信息事件
2. **前端**: 监听事件 → 检测引用 → 更新状态
3. **UI**: 多层次可视化（会话级 + 消息级 + 文本级）

所有代码已通过编译测试，可以直接运行使用。用户现在可以清楚地知道AI是否真正使用了知识库内容，大大提升了RAG功能的透明度和可信度。
