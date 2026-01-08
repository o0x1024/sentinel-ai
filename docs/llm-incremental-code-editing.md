# LLM 增量代码编辑方案

## 问题描述

之前 LLM 在修改代码时会返回完整的文件内容,即使用户只想修改一小部分代码。这导致:
1. 响应速度慢
2. Token 消耗大
3. 容易引入不必要的改动
4. 用户体验差

## 解决方案

实现了类似 Cursor/Aider 的 SEARCH/REPLACE 增量编辑格式,让 LLM 只返回需要修改的代码片段。

## 核心机制

### 1. Prompt 约束

在系统 prompt 中明确要求 LLM 使用特定的输出格式:

```markdown
## CRITICAL OUTPUT FORMAT

You MUST use one of these formats:

### Format 1: SEARCH/REPLACE (for modifying existing code)
```diff
<<<<<<< SEARCH
[exact code to find - must be unique, include 3-5 lines before and after]
=======
[new code to replace with]
>>>>>>> REPLACE
```

### Format 2: INSERT (for adding new code)
```diff
<<<<<<< INSERT_AFTER
[exact line after which to insert]
=======
[new code to insert]
>>>>>>> INSERT
```

### Format 3: FULL CODE (ONLY if user explicitly requests)
```typescript
[complete code]
```
```

### 2. 自动解析和应用

系统会自动:
1. 解析 LLM 返回的 diff 格式
2. 验证 SEARCH 块是否存在且唯一
3. 自动应用到编辑器
4. 显示成功/失败通知

## 使用示例

### 示例 1: 添加新的 payload

**用户请求:**
```
增加针对 ORDER BY 注入的 payload
```

**LLM 响应:**
```diff
<<<<<<< SEARCH
const sqlPayloads = [
    "' OR '1'='1",
    "' UNION SELECT NULL--"
];
=======
const sqlPayloads = [
    "' OR '1'='1",
    "' UNION SELECT NULL--",
    "' ORDER BY 1--",
    "' ORDER BY 10--",
    "' ORDER BY 100--"
];
>>>>>>> REPLACE
```

**结果:** 系统自动找到 `sqlPayloads` 数组并添加新的 payload,无需返回整个文件。

### 示例 2: 插入新函数

**用户请求:**
```
在 validateInput 函数后添加一个 sanitizePayload 函数
```

**LLM 响应:**
```diff
<<<<<<< INSERT_AFTER
export function validateInput(input: string): boolean {
    return input && input.length > 0;
}
=======

export function sanitizePayload(payload: string): string {
    return payload.replace(/[<>'"]/g, '');
}
>>>>>>> INSERT
```

**结果:** 系统自动在指定位置插入新函数。

## 技术实现

### 核心函数

1. **parseDiffBlocks(content: string)**: 解析 LLM 响应中的 diff 块
2. **applyDiffBlocks(currentCode: string, blocks: DiffBlock[])**: 应用 diff 到当前代码

### 错误处理

- 如果 SEARCH 文本不存在: 提示无法找到
- 如果 SEARCH 文本不唯一: 提示需要更多上下文
- 如果无法自动应用: 回退到手动复制模式

## 优势

1. **速度快**: 只返回需要修改的部分,减少 Token 消耗
2. **精确**: 明确指定修改位置,避免意外改动
3. **可验证**: 自动检查 SEARCH 块的唯一性
4. **用户友好**: 自动应用,无需手动复制粘贴
5. **兼容性**: 如果 LLM 不遵守格式,自动回退到全量代码模式

## 最佳实践

### 对于 LLM

1. **默认使用 SEARCH/REPLACE**: 除非用户明确要求完整代码
2. **包含足够上下文**: SEARCH 块应包含 3-5 行前后文,确保唯一性
3. **一次一个改动**: 多个改动使用多个 SEARCH/REPLACE 块
4. **先解释再给代码**: 简短说明要做什么,然后提供 diff

### 对于用户

1. **明确需求**: 说清楚要修改哪个部分
2. **提供上下文**: 如果可能,选中相关代码再提问
3. **验证结果**: 自动应用后检查是否符合预期

## 扩展方向

1. **支持多文件修改**: 扩展格式支持跨文件的 diff
2. **可视化 diff**: 在 UI 中高亮显示修改的部分
3. **撤销/重做**: 支持撤销自动应用的修改
4. **冲突解决**: 如果代码已被修改,智能合并或提示冲突

## 参考项目

- [Aider](https://github.com/paul-gauthier/aider): AI pair programming tool
- [Cursor](https://cursor.sh): AI-first code editor
- [Continue](https://github.com/continuedev/continue): Open-source Copilot alternative
