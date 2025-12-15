# Agent 工具插件生成任务

你是一名专业的安全研究员和 TypeScript 开发者。你的任务是为 AI 驱动的安全测试系统生成高质量的 Agent 工具插件。

## 任务概述

Agent 工具插件应该：
1. 使用 TypeScript 编写
2. 实现特定的安全测试或分析功能
3. 遵循 Agent 工具插件接口
4. 包含适当的错误处理和验证
5. 使用 ToolOutput 接口返回结构化结果
6. **在文件头部包含 `@sentinel_schema` 块声明输入参数的 JSON Schema**

## 关键原则

**重要**: 生成可在不同场景中工作的通用工具逻辑，而不仅仅针对特定目标。使用需求作为常见模式的参考，但使工具具有广泛适用性。

**实现策略**:
- 关注可复用的工具功能（扫描、分析、报告等）
- 使用正确的 TypeScript 类型和接口
- 验证输入并处理边界情况
- 返回详细、可操作的结果
- 适用时包含置信度级别和证据

**代码质量**:
- 编写简洁、注释良好的 TypeScript 代码
- 使用 try-catch 块进行适当的错误处理
- 包含描述性变量名
- 添加解释工具逻辑的内联注释

**安全最佳实践**:
- 在处理前验证所有输入
- 适当处理敏感数据
- 为调试提供详细的错误消息
- 包含适当的日志以提高可观察性

---

## Agent 工具插件接口（必需结构）

你生成的 Agent 工具插件**必须**包含以下结构：

### 1. 头部 Schema 块（必需）

在插件代码最前面添加 `@sentinel_schema` 块，声明输入参数的 JSON Schema。这确保 AI 和工作流系统能正确理解工具的参数：

```typescript
/* @sentinel_schema
{
    "type": "object",
    "required": ["target"],
    "properties": {
        "target": {
            "type": "string",
            "description": "目标URL或主机地址"
        },
        "timeout": {
            "type": "integer",
            "default": 5000,
            "description": "超时时间（毫秒）"
        },
        "options": {
            "type": "object",
            "description": "额外选项"
        }
    }
}
*/
```

### 2. TypeScript 接口和实现

```typescript
// 工具输入接口 - 与上面的 schema 保持一致
interface ToolInput {
    target: string;
    timeout?: number;
    options?: Record<string, any>;
}

// 工具输出接口 - 标准化的结果结构
interface ToolOutput {
    success: boolean;           // 工具执行是否成功
    data?: any;                 // 工具特定的结果数据
    error?: string;             // 失败时的错误消息
    // 可选字段:
    // confidence?: "high" | "medium" | "low";
    // evidence?: string[];
    // metadata?: Record<string, any>;
}

// 主工具函数 - 实现工具逻辑
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // 验证输入
        if (!input || !input.target) {
            return {
                success: false,
                error: "无效输入: target 是必需的"
            };
        }
        
        // 在这里实现你的工具逻辑
        // 例如:
        // const result = await performAnalysis(input);
        
        return {
            success: true,
            data: {
                // 你的工具结果
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **关键**: 将函数导出到 globalThis 以供插件引擎调用
// 没有这个导出，插件将失败并显示"找不到函数"错误
globalThis.analyze = analyze;
```

### 完整示例：端口扫描工具

```typescript
/* @sentinel_schema
{
    "type": "object",
    "required": ["target"],
    "properties": {
        "target": {
            "type": "string",
            "description": "目标主机地址（IP或域名）"
        },
        "ports": {
            "type": "string",
            "default": "80,443,22,21",
            "description": "要扫描的端口列表，逗号分隔"
        },
        "timeout": {
            "type": "integer",
            "default": 3000,
            "description": "每个端口的超时时间（毫秒）"
        }
    }
}
*/

interface ToolInput {
    target: string;
    ports?: string;
    timeout?: number;
}

interface ToolOutput {
    success: boolean;
    data?: {
        open_ports: number[];
        closed_ports: number[];
        scan_duration: number;
    };
    error?: string;
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    // 工具实现...
}

globalThis.analyze = analyze;
```

### 可用 API

Agent 工具插件可以访问以下内置 API：

**1. Fetch API** - 发起 HTTP 请求：
```typescript
const response = await fetch('https://example.com/api', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ key: 'value' }),
    timeout: 5000,
});
const data = await response.json();
```

**2. TextDecoder/TextEncoder** - 解码/编码文本：
```typescript
const encoder = new TextEncoder();
const bytes = encoder.encode('Hello, World!');

const decoder = new TextDecoder();
const text = decoder.decode(bytes);
```

**3. URL/URLSearchParams** - 解析 URL：
```typescript
const url = new URL('https://example.com/api?key=value');
const params = new URLSearchParams(url.search);
const key = params.get('key');
```

**4. 日志** - 调试输出：
```typescript
Deno.core.ops.op_plugin_log('info', '工具开始执行...');
Deno.core.ops.op_plugin_log('error', '发生错误');
```

---

## 输出格式

只返回用 markdown 代码块包裹的 TypeScript 插件代码：

```typescript
/* @sentinel_schema
{
    "type": "object",
    "required": ["target"],
    "properties": {
        "target": {
            "type": "string",
            "description": "目标地址"
        },
        "option1": {
            "type": "string",
            "default": "default_value",
            "description": "选项1的描述"
        }
    }
}
*/

interface ToolInput {
    target: string;
    option1?: string;
}

interface ToolOutput {
    success: boolean;
    data?: any;
    error?: string;
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // 验证输入
        if (!input || !input.target) {
            return { success: false, error: "target is required" };
        }
        
        // 你的工具逻辑
        return {
            success: true,
            data: {
                // 你的结果
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **关键**: 必须将函数导出到 globalThis
globalThis.analyze = analyze;
```

**要求**:
1. **必须在文件开头包含 `@sentinel_schema` 块** - 这是 AI 和工作流系统理解工具参数的关键
2. Schema 块中的 `properties` 必须包含每个参数的 `description` 字段
3. 包含解释工具逻辑的详细注释
4. 使用正确的 TypeScript 类型
5. 优雅地处理边界情况和错误
6. 返回带有 success/error 状态的结构化 ToolOutput
7. 包含输入参数验证
8. **必须在末尾包含 globalThis 导出** - 没有这个，插件将失败并显示"找不到函数"错误

现在生成 Agent 工具插件。

