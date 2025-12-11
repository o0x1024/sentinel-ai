# 知识库引用检测 - 快速参考

## 一分钟了解

当你开启知识库功能对话时，系统会：
1. ✅ 自动检索相关知识
2. ✅ AI使用 `[SOURCE n]` 格式引用
3. ✅ 显示是否真的引用了知识库

## 视觉标识

### 📖 知识库标签
- **位置**: 会话标题右侧
- **含义**: RAG功能已启用
- **颜色**: 蓝色

### 引用状态横幅
- **位置**: AI消息顶部
- **两种状态**:
  - ✅ `已引用知识库 (3 处引用)` - 成功引用
  - ⚠️ `已启用知识库，但未找到相关内容` - 无相关内容

### [SOURCE n] 标记
- **样式**: 蓝色背景，等宽字体
- **交互**: 鼠标悬停显示提示
- **含义**: 引用知识库第n条证据

## 使用步骤

```
1. 创建并激活知识库集合
   ↓
2. 点击 🧠 图标开启RAG
   ↓
3. 提问
   ↓
4. 查看引用状态和标记
```

## 示例对话

### ✅ 成功引用
```
用户: 什么是XSS攻击？

AI: XSS是一种Web安全漏洞 [SOURCE 1]。攻击者可以注入
恶意脚本 [SOURCE 2]...

📖 已引用知识库 (2 处引用)
```

### ⚠️ 无相关内容
```
用户: 今天天气怎么样？

AI: 抱歉，我无法获取实时天气信息...

📖 已启用知识库，但未找到相关内容
```

## 关键文件

| 文件 | 功能 |
|------|------|
| `src-tauri/src/commands/ai.rs` | 后端RAG逻辑 |
| `src/composables/useAgentEvents.ts` | 前端检测逻辑 |
| `src/components/Agent/MessageBlock.vue` | 引用状态显示 |
| `src/components/Agent/MarkdownRenderer.vue` | 引用高亮 |

## 技术要点

### 检测正则
```typescript
/\[SOURCE\s+\d+\]/gi
```

### 元信息结构
```typescript
{
  rag_applied: boolean,
  rag_sources_used: boolean,
  source_count: number
}
```

### 事件流
```
ai_meta_info → rag_applied=true
    ↓
assistant_message_saved → 检测[SOURCE n]
    ↓
更新 rag_sources_used 和 source_count
```

## 故障排查

| 问题 | 解决方案 |
|------|---------|
| 没有引用 | 检查知识库是否包含相关内容 |
| 引用不准确 | 改进文档质量，调整相似度阈值 |
| 性能慢 | 降低top_k值，优化知识库大小 |

## 相关文档

- 📖 [完整技术文档](./rag-citation-detection.md)
- 📚 [使用示例](./rag-usage-example.md)
- 📝 [实现总结](./rag-citation-summary.md)
