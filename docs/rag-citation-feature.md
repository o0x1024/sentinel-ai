# 知识库引用检测功能

## 功能说明

当用户开启知识库检索增强(RAG)功能进行对话时，系统能够：
1. **自动检测** AI是否真正引用了知识库中的内容
2. **可视化显示** 引用状态和引用次数
3. **高亮标记** 具体的引用位置 `[SOURCE n]`

这大大提升了RAG功能的**透明度**和**可信度**。

## 核心特性

### 🎯 智能检测
- 区分"知识库已启用"和"真正引用了知识库"
- 自动统计引用次数
- 实时检测和更新状态

### 👁️ 可视化
- **会话级**: 显示知识库启用状态
- **消息级**: 显示引用状态横幅
- **文本级**: 高亮显示 `[SOURCE n]` 标记

### 🔍 可追溯
- 每个引用都有明确的标记
- 悬停显示提示信息
- 未来支持点击查看原文

## 快速开始

### 1. 准备知识库
```
工具 → 知识库管理 → 创建集合 → 上传文档 → 激活
```

### 2. 开启RAG
```
AI助手 → 点击 🧠 图标 → 图标高亮 → 开始对话
```

### 3. 查看引用
```
AI回复 → 查看引用状态 → 注意 [SOURCE n] 标记
```

## 使用示例

### 示例1: 成功引用知识库

**对话**:
```
👤 用户: 什么是SQL注入攻击？

🤖 AI: SQL注入是一种常见的Web应用安全漏洞 [SOURCE 1]。
攻击者通过在输入字段中插入恶意SQL代码，可以绕过应用
程序的安全验证 [SOURCE 2]。

防护措施包括：
1. 使用参数化查询 [SOURCE 1]
2. 严格验证用户输入 [SOURCE 3]
3. 最小权限原则 [SOURCE 2]

📖 已引用知识库 (3 处引用)
```

**效果**:
- ✅ `[SOURCE 1]`, `[SOURCE 2]`, `[SOURCE 3]` 被蓝色高亮
- ✅ 消息顶部显示"已引用知识库 (3 处引用)"
- ✅ 会话标题旁显示 📖 知识库 标签

### 示例2: 无相关内容

**对话**:
```
👤 用户: 今天天气怎么样？

🤖 AI: 抱歉，我无法获取实时天气信息。我的知识库
主要包含网络安全相关内容。

📖 已启用知识库，但未找到相关内容
```

**效果**:
- ⚠️ 没有 `[SOURCE n]` 标记
- ⚠️ 显示"已启用知识库，但未找到相关内容"
- ✅ 会话标题旁仍显示 📖 知识库 标签

## 技术实现

### 架构图

```
┌─────────────────────────────────────────────────────────┐
│                      用户界面                            │
├─────────────────────────────────────────────────────────┤
│  会话头部: 📖 知识库                                     │
│  ├─ AgentView.vue                                       │
│  │                                                       │
│  消息显示: 📖 已引用知识库 (n 处引用)                   │
│  ├─ MessageBlock.vue                                    │
│  │                                                       │
│  引用高亮: [SOURCE 1] [SOURCE 2]                        │
│  └─ MarkdownRenderer.vue                                │
└─────────────────────────────────────────────────────────┘
                          ↑
                          │ 事件监听
                          │
┌─────────────────────────────────────────────────────────┐
│                   前端检测逻辑                           │
├─────────────────────────────────────────────────────────┤
│  useAgentEvents.ts                                      │
│  ├─ 监听 ai_meta_info 事件                              │
│  ├─ 检测 [SOURCE n] 模式                                │
│  ├─ 统计引用次数                                        │
│  └─ 更新 ragMetaInfo                                    │
└─────────────────────────────────────────────────────────┘
                          ↑
                          │ Tauri事件
                          │
┌─────────────────────────────────────────────────────────┐
│                    后端RAG处理                           │
├─────────────────────────────────────────────────────────┤
│  commands/ai.rs :: agent_execute                        │
│  ├─ 检索知识库内容                                      │
│  ├─ 注入知识溯源规范                                    │
│  ├─ 要求使用 [SOURCE n] 格式                            │
│  └─ 发送 ai_meta_info 事件                              │
└─────────────────────────────────────────────────────────┘
```

### 关键代码

#### 后端 (Rust)
```rust
// src-tauri/src/commands/ai.rs
let policy = "你必须严格基于证据回答问题。在回答中引用证据时，使用 [SOURCE n] 格式。";
let augmented = format!("[知识溯源规范]\n{}\n\n[证据块]\n{}", policy, context);

// 发送元信息事件
app_handle.emit("ai_meta_info", &serde_json::json!({
    "conversation_id": conv_id,
    "message_id": msg_id,
    "rag_applied": true
}));
```

#### 前端 (TypeScript)
```typescript
// src/composables/useAgentEvents.ts
const sourcePattern = /\[SOURCE\s+\d+\]/gi
const matches = payload.content.match(sourcePattern)
if (matches && matches.length > 0) {
  ragMetaInfo.value = {
    rag_applied: true,
    rag_sources_used: true,
    source_count: matches.length
  }
}
```

#### UI (Vue)
```vue
<!-- src/components/Agent/MessageBlock.vue -->
<div v-if="ragInfo" class="rag-indicator">
  <i class="fas fa-book text-info"></i>
  <span v-if="ragInfo.rag_sources_used">
    已引用知识库 ({{ ragInfo.source_count }} 处引用)
  </span>
  <span v-else>
    已启用知识库，但未找到相关内容
  </span>
</div>
```

## 文件清单

### 修改的文件 (5个)

| 文件 | 变更内容 |
|------|---------|
| `src/composables/useAgentEvents.ts` | 添加RAG检测逻辑 |
| `src/components/Agent/MessageBlock.vue` | 添加引用状态显示 |
| `src/components/Agent/MarkdownRenderer.vue` | 添加引用高亮 |
| `src/components/Agent/AgentView.vue` | 添加知识库标签 |
| `src/components/InputAreaComponent.vue` | 更新提示文本 |

### 新增的文件 (4个)

| 文件 | 说明 |
|------|------|
| `docs/rag-citation-detection.md` | 技术实现详解 |
| `docs/rag-usage-example.md` | 使用示例和最佳实践 |
| `docs/rag-citation-summary.md` | 实现总结 |
| `docs/rag-citation-quickref.md` | 快速参考卡片 |

## 测试状态

### ✅ 编译测试
```bash
# 前端
npm run build
# ✅ 成功，无错误

# 后端
cargo check
# ✅ 成功，仅有警告
```

### 功能测试场景

| 场景 | 状态 |
|------|------|
| 成功引用知识库 | ✅ 待测试 |
| 无相关内容 | ✅ 待测试 |
| 未开启RAG | ✅ 待测试 |
| 引用高亮显示 | ✅ 待测试 |
| 悬停提示 | ✅ 待测试 |

## 用户价值

### 对用户的好处

1. **透明度** 📊
   - 清楚知道AI是否使用了知识库
   - 了解引用了多少次
   - 看到具体的引用位置

2. **可信度** ✅
   - 区分基于知识库的回答和通用回答
   - 验证AI的回答有据可依
   - 评估知识库的有效性

3. **可控性** 🎮
   - 根据引用情况调整知识库
   - 优化文档质量
   - 改进检索效果

### 对开发者的好处

1. **可维护性** 🔧
   - 清晰的代码结构
   - 完善的文档
   - 易于扩展

2. **可调试性** 🐛
   - 实时查看RAG状态
   - 追踪引用检测过程
   - 快速定位问题

3. **可扩展性** 🚀
   - 预留引用详情接口
   - 支持多种引用格式
   - 易于添加新功能

## 未来规划

### 短期 (1-2周)
- [ ] 引用详情弹窗
- [ ] 引用来源列表
- [ ] 引用质量评分

### 中期 (1-2月)
- [ ] 引用管理功能
- [ ] 引用统计分析
- [ ] 多知识库支持

### 长期 (3-6月)
- [ ] 智能引用推荐
- [ ] 引用网络图谱
- [ ] 协同标注系统

## 相关资源

### 文档
- 📖 [技术实现详解](./rag-citation-detection.md)
- 📚 [使用示例](./rag-usage-example.md)
- 📝 [实现总结](./rag-citation-summary.md)
- 🎯 [快速参考](./rag-citation-quickref.md)

### 代码
- 后端: `src-tauri/src/commands/ai.rs`
- 前端: `src/composables/useAgentEvents.ts`
- UI: `src/components/Agent/`

## 常见问题

### Q: 为什么有时候开启RAG但没有引用？
A: 可能是知识库中没有相关内容，或者相似度阈值设置过高。

### Q: 如何提高引用的准确性？
A: 改进知识库文档质量，使用更准确的关键词，启用重排序功能。

### Q: [SOURCE n] 标记可以自定义吗？
A: 当前版本固定为此格式，未来版本会支持自定义。

### Q: 如何查看引用的原文？
A: 当前版本暂不支持，已列入短期规划。

## 贡献指南

欢迎贡献代码和文档！

### 开发环境
```bash
# 前端
npm install
npm run dev

# 后端
cd src-tauri
cargo build
```

### 提交规范
```
feat: 添加引用详情弹窗
fix: 修复引用检测正则表达式
docs: 更新RAG使用文档
```

## 许可证

本项目遵循主项目的许可证。

---

**最后更新**: 2025-12-10  
**版本**: 1.0.0  
**状态**: ✅ 已完成，待测试
