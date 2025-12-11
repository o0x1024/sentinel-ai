# 知识库引用检测功能说明

## 功能概述

当用户开启知识库检索增强(RAG)功能进行对话时,系统能够自动检测AI是否真正引用了知识库中的内容。

## 工作原理

### 1. 后端处理流程

在 `src-tauri/src/commands/ai.rs` 的 `agent_execute` 函数中:

```rust
// 当启用RAG时,系统会在系统提示中注入知识溯源规范
let policy = "你必须严格基于证据回答问题。在回答中引用证据时，使用 [SOURCE n] 格式。如果证据不足，请直接回答并避免编造。";
let augmented = format!("{}\n\n[知识溯源规范]\n{}\n\n[证据块]\n{}", base, policy, context);
```

- RAG启用时,会从知识库检索相关内容作为"证据块"
- 要求AI在引用证据时使用 `[SOURCE n]` 格式
- 发送 `ai_meta_info` 事件告知前端RAG已应用

### 2. 前端检测逻辑

在 `src/composables/useAgentEvents.ts` 中:

```typescript
// 监听元信息事件
if (payload.rag_applied) {
  ragMetaInfo.value = {
    rag_applied: true,
    rag_sources_used: false, // 初始为false
    source_count: 0
  }
}

// 在收到AI响应时检测引用
const sourcePattern = /\[SOURCE\s+\d+\]/gi
const matches = payload.content.match(sourcePattern)
if (matches && matches.length > 0) {
  ragMetaInfo.value.rag_sources_used = true
  ragMetaInfo.value.source_count = matches.length
}
```

### 3. UI显示

#### 3.1 会话头部指示器
在 `AgentView.vue` 中显示知识库启用状态:
```vue
<div v-if="ragEnabled" class="flex items-center gap-1 px-2 py-1 bg-info/10">
  <i class="fas fa-book text-info"></i>
  <span class="text-xs text-info">知识库</span>
</div>
```

#### 3.2 消息级别指示器
在 `MessageBlock.vue` 中显示引用状态:
```vue
<div v-if="ragInfo" class="rag-indicator">
  <template v-if="ragInfo.rag_sources_used">
    已引用知识库 ({{ ragInfo.source_count }} 处引用)
  </template>
  <template v-else>
    已启用知识库，但未找到相关内容
  </template>
</div>
```

#### 3.3 引用高亮显示
在 `MarkdownRenderer.vue` 中高亮显示 `[SOURCE n]`:
```typescript
content = content.replace(
  /\[SOURCE\s+(\d+)\]/gi,
  '<span class="source-citation" title="知识库引用 #$1">[SOURCE $1]</span>'
)
```

样式效果:
- 蓝色背景和边框
- 等宽字体
- 悬停时有提示和动画效果

## 使用场景

### 场景1: 成功引用知识库
1. 用户开启知识库功能
2. 提问: "什么是XSS攻击?"
3. AI回复: "XSS(跨站脚本攻击)是一种常见的Web安全漏洞 [SOURCE 1]。攻击者通过注入恶意脚本... [SOURCE 2]"
4. 系统显示: "已引用知识库 (2 处引用)"

### 场景2: 知识库无相关内容
1. 用户开启知识库功能
2. 提问: "今天天气怎么样?"
3. AI回复: "抱歉,我无法获取实时天气信息..."
4. 系统显示: "已启用知识库，但未找到相关内容"

### 场景3: 未开启知识库
1. 用户未开启知识库功能
2. 正常对话
3. 不显示任何知识库相关指示器

## 数据流

```
用户开启RAG → agent_execute
                    ↓
              检索知识库内容
                    ↓
              注入系统提示(要求使用[SOURCE n])
                    ↓
              发送 ai_meta_info 事件
                    ↓
              前端监听事件,标记 rag_applied=true
                    ↓
              AI流式响应
                    ↓
              前端检测 [SOURCE n] 模式
                    ↓
              更新 rag_sources_used 和 source_count
                    ↓
              在UI中显示引用状态和高亮引用标记
```

## 技术细节

### 引用格式正则表达式
```typescript
const sourcePattern = /\[SOURCE\s+\d+\]/gi
```
- 匹配 `[SOURCE 1]`, `[SOURCE 2]` 等格式
- 不区分大小写 (gi 标志)
- 允许 SOURCE 和数字之间有空格

### 元信息结构
```typescript
interface RagMetaInfo {
  rag_applied: boolean        // RAG是否已应用
  rag_sources_used: boolean   // 是否实际引用了知识库
  source_count: number        // 引用次数
}
```

### 消息元数据
```typescript
{
  id: string
  type: 'final'
  content: string
  timestamp: number
  metadata: {
    rag_info: RagMetaInfo  // 附加RAG信息
  }
}
```

## 优势

1. **透明性**: 用户可以清楚地知道AI是否使用了知识库
2. **可追溯**: 通过 `[SOURCE n]` 标记可以追溯到具体的知识来源
3. **可靠性**: 区分"知识库已启用"和"真正引用了知识库"
4. **用户体验**: 视觉化的引用高亮和状态指示器

## 未来改进方向

1. **引用详情**: 点击 `[SOURCE n]` 显示原始文档片段
2. **引用管理**: 在侧边栏显示所有引用的来源列表
3. **引用质量**: 评估引用的相关性和准确性
4. **引用统计**: 统计知识库使用频率和效果
