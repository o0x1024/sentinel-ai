# ReAct 架构集成指南

## 概述

ReAct（Reasoning and Acting）架构已成功集成到 Sentinel AI 系统中。该架构通过迭代的"思考-行动-观察"循环实现自适应决策，特别适合探索性和需要灵活推理的任务。

## 架构特点

### 核心优势
- **交替推理执行**：在每次迭代中交替进行思考和行动
- **自适应决策**：根据观察结果动态调整策略
- **良好可解释性**：完整的思考过程可追溯
- **灵活工具调用**：支持 MCP 工具动态执行

### 适用场景
- 探索性任务（调试、信息收集）
- 多步推理（研究、分析）
- 不确定性环境（需要根据反馈调整）
- 需要可解释性的任务

## 系统集成

### 1. 后端实现

#### 核心模块
```
src-tauri/src/engines/react/
├── types.rs           # 类型定义
├── parser.rs          # Action 解析器
├── executor.rs        # ReAct 执行器
├── engine_adapter.rs  # 引擎适配器
└── mod.rs            # 模块导出
```

#### Tauri 命令
```rust
// 执行 ReAct 任务
execute_react_task(request: ExecuteReactRequest) -> CommandResponse<ExecuteReactResponse>

// 获取配置
get_react_config() -> CommandResponse<ReactConfig>

// 更新配置
update_react_config(config: ReactConfig) -> CommandResponse<bool>
```

### 2. 前端集成

#### ArchitectureSelector.vue
已添加 ReAct 选项到架构选择器：
- 绿色标识
- 中等复杂度/速度/资源
- "灵活适应"标签

#### 配置参数
```typescript
{
  max_iterations: 10,        // 最大迭代次数
  temperature: 0.7,          // LLM 温度
  max_tokens: 2000,          // 最大 token 数
  enable_rag: true,          // 启用 RAG
  verbose: true,             // 详细日志
  parse_strategy: "json"     // 解析策略
}
```

### 3. 智能调度器

#### 选择规则
```rust
// 优先场景
- 调试任务
- 探索任务
- 信息收集
- 中等复杂度分析（步骤 ≤ 5）
- 研究理解类任务

// 性能特征
token_efficiency: 60%
execution_speed: 50%
resource_usage: 50%
parallel_efficiency: 10%  // 顺序执行
```

## 使用示例

### 前端调用
```typescript
import { invoke } from '@tauri-apps/api/tauri'

// 执行 ReAct 任务
const response = await invoke('execute_react_task', {
  request: {
    task: "分析这个系统的安全漏洞",
    config: {
      max_iterations: 15,
      enable_rag: true,
      verbose: true
    },
    conversation_id: "conv-123",
    message_id: "msg-456"
  }
})

console.log(response.data)
// {
//   trace_id: "react-1234567890",
//   status: "completed",
//   answer: "发现以下安全漏洞...",
//   iterations: 8,
//   tool_calls: 5,
//   duration_ms: 45000
// }
```

### 流式消息监听
```typescript
import { listen } from '@tauri-apps/api/event'

// 监听推理步骤
listen('stream-message', (event) => {
  const message = event.payload
  switch (message.type) {
    case 'Reasoning':
      console.log('思考:', message.content)
      break
    case 'ToolUpdate':
      console.log('工具调用:', message.content)
      break
    case 'FinalResult':
      console.log('最终结果:', message.content)
      break
  }
})
```

## 执行流程

### 1. 迭代循环
```
1. Thought（思考）
   ↓
2. Action（行动）- 解析为工具调用或最终答案
   ↓
3. Observation（观察）- 工具执行结果
   ↓
4. 收敛检查 - 是否达到最终答案？
   ├─ 是 → 返回结果
   └─ 否 → 回到步骤 1
```

### 2. Action 解析
支持两种格式：

**JSON 格式（优先）**
```json
{
  "action": "tool_name",
  "action_input": {"param": "value"}
}
```

**自然语言格式（回退）**
```
Action: search_database
Action Input: query for vulnerabilities
```

### 3. 收敛条件
- 检测到 Final Answer
- 达到最大迭代次数
- LLM 返回错误
- 工具执行失败（超过重试次数）

## 配置优化

### 性能调优
```rust
ReactConfig {
    max_iterations: 15,        // 复杂任务增加
    temperature: Some(0.5),    // 降低随机性
    max_tokens: Some(3000),    // 更长的推理
    enable_rag: true,          // 启用知识增强
    verbose: false,            // 生产环境关闭
    parse_strategy: ParseStrategy::JsonFirst, // 默认策略
}
```

### 超时设置
```rust
// 每次迭代 30 秒超时
timeout: Some(max_iterations as u64 * 30000)
```

## 监控与调试

### 日志级别
```rust
RUST_LOG=info  // 基本信息
RUST_LOG=debug // 详细执行流程
RUST_LOG=trace // 完整 LLM 交互
```

### 关键指标
- `iterations`: 实际迭代次数
- `tool_calls`: 工具调用次数
- `duration_ms`: 总执行时间
- `status`: 完成状态（completed/failed/max_iterations）

## 已知限制

1. **顺序执行**：不支持并行工具调用
2. **Token 消耗**：多轮对话导致较高 token 使用
3. **收敛依赖**：依赖 LLM 正确格式化输出
4. **工具依赖**：需要 MCP 服务正常运行

## 下一步

### 待完成任务
- [ ] 集成测试覆盖
- [ ] 性能基准测试
- [ ] UI 端到端测试
- [ ] 错误恢复机制优化
- [ ] RAG 上下文增强实现

### 改进方向
1. 增加中间检查点保存
2. 支持人工介入决策
3. 优化 Action 解析准确率
4. 添加执行预算控制
5. 实现执行轨迹可视化

## 参考资料

- [ReAct 论文](https://arxiv.org/abs/2210.03629)
- [Sentinel AI 架构文档](./architecture.md)
- [MCP 工具集成](./mcp_integration.md)
