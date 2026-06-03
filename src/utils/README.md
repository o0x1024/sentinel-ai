# Shell Highlight Utility

## 概述

`shellHighlight.ts` 提供了基于 CodeMirror 的 shell 命令语法高亮功能。

## 特性

- ✅ 使用 CodeMirror 的 shell 语法引擎
- ✅ 支持完整的 bash/shell 语法
- ✅ 与项目的 oneDark 主题配色一致
- ✅ 轻量级实现，不创建完整编辑器实例
- ✅ 自动 HTML 转义，安全防护 XSS
- ✅ 零额外依赖（复用现有的 CodeMirror）

## 使用方法

### 基本用法

```typescript
import { highlightShellCommand } from '@/utils/shellHighlight'

const command = 'ls -la | grep pattern'
const highlightedHtml = highlightShellCommand(command)

// 在模板中使用 v-html 渲染
<div v-html="highlightedHtml"></div>
```

### 在 Vue 组件中使用

```vue
<template>
  <div class="command-display">
    <span v-html="highlightedCommand"></span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { highlightShellCommand } from '@/utils/shellHighlight'

const props = defineProps<{
  command: string
}>()

const highlightedCommand = computed(() => 
  highlightShellCommand(props.command)
)
</script>

<style scoped>
/* 引入 CodeMirror 样式 */
:deep(.cm-keyword) { color: #c678dd; }
:deep(.cm-string) { color: #98c379; }
:deep(.cm-variable) { color: #e06c75; }
/* ... 其他样式 */
</style>
```

## 支持的语法元素

- **命令关键字**: `cd`, `ls`, `cat`, `grep` 等
- **字符串**: 单引号和双引号字符串
- **变量**: `$VAR`, `${VAR}` 等
- **操作符**: `|`, `>`, `<`, `>>`, `&&`, `||` 等
- **选项**: `-l`, `--help` 等
- **注释**: `# comment`
- **数字**: 数值参数
- **内置命令**: shell 内置命令

## 配色方案

使用 CodeMirror oneDark 主题配色：

| 元素 | 颜色 | 说明 |
|------|------|------|
| 命令关键字 | `#c678dd` | 紫色 |
| 字符串 | `#98c379` | 绿色 |
| 变量 | `#e06c75` | 红色 |
| 操作符 | `#56b6c2` | 青色 |
| 数字 | `#d19a66` | 橙色 |
| 注释 | `#5c6370` | 灰色 |

## 安全性

- 自动进行 HTML 转义，防止 XSS 攻击
- 所有用户输入都会被安全处理
- 可以安全地使用 `v-html` 渲染

## 性能

- 轻量级实现，只进行语法分析
- 不创建完整的编辑器实例
- 适合大量命令的场景
- 可以考虑添加缓存优化（如需要）

## 示例

### 简单命令
```bash
ls -la /home
```

### 管道命令
```bash
cat file.txt | grep pattern | wc -l
```

### 复杂 CTF 命令
```bash
python3 -c "import base64; print(base64.b64decode('dGVzdA=='))"
```

### 带变量的命令
```bash
echo $HOME && cd $HOME/projects
```

## 测试

运行测试：
```bash
npm test -- src/utils/shellHighlight.test.ts
```

## 相关文件

- `src/utils/shellHighlight.ts` - 主要实现
- `src/utils/shellHighlight.test.ts` - 单元测试
- `src/components/Agent/ShellToolResult.vue` - 使用示例
