# Agent插件工具 - 快速开始指南

## 测试Agent插件工具集成

现在所有代码已经完成并编译通过。以下是测试和使用的步骤：

## 1. 创建测试插件

首先需要在数据库中创建一个 `category = "agentTools"` 的测试插件。

### 方法1：通过UI创建

1. 打开 **PluginManagement.vue**
2. 点击"新建插件"
3. 填写插件信息：
   - **类别**: 选择 "agentTools"
   - **ID**: `test.hello_agent`
   - **名称**: `Agent问候插件`
   - **描述**: `一个测试插件，用于验证Agent插件工具集成`
4. 编写插件代码：

```typescript
export default {
  id: 'test.hello_agent',
  name: 'Agent问候插件',
  category: 'agentTools',
  version: '1.0.0',
  description: '测试Agent调用插件的功能',
  
  async onRequest(ctx: RequestContext): Promise<Finding[]> {
    // 解析Agent传入的参数
    const bodyText = new TextDecoder().decode(ctx.body);
    const body = JSON.parse(bodyText);
    
    const target = body.inputs?.target || ctx.url;
    const context = body.context || {};
    const data = body.data || {};
    
    // 构建响应
    const findings: Finding[] = [{
      vuln_type: 'info',
      severity: 'info',
      title: 'Agent插件调用成功',
      description: `插件收到Agent调用！目标: ${target}`,
      evidence: {
        context: context,
        data: data,
        message: '这是从插件返回的测试消息',
      },
      url: target,
      timestamp: new Date().toISOString(),
    }];
    
    return findings;
  }
}
```

5. 保存并**启用**插件

### 方法2：直接插入数据库

```sql
INSERT INTO passive_plugin_registry (
    id, name, version, author, category, description, 
    default_severity, tags, file_path, file_hash, enabled
) VALUES (
    'test.hello_agent',
    'Agent问候插件',
    '1.0.0',
    'Sentinel AI',
    'agentTools',
    '测试Agent调用插件的功能',
    'info',
    '["test", "agent"]',
    'plugins/test.hello_agent.ts',
    'mock_hash_123',
    1
);
```

## 2. 验证插件工具注册

### 使用前端测试命令

打开浏览器控制台，执行：

```javascript
// 1. 列出所有插件工具
const tools = await window.__TAURI__.invoke('list_agent_plugin_tools');
console.log('Available plugin tools:', tools);
// 应该看到: ["plugin::test.hello_agent"]

// 2. 获取插件工具详情
const info = await window.__TAURI__.invoke('get_plugin_tool_info', {
    pluginId: 'test.hello_agent'
});
console.log('Plugin tool info:', info);

// 3. 测试执行插件工具
const result = await window.__TAURI__.invoke('test_execute_plugin_tool', {
    request: {
        plugin_id: 'test.hello_agent',
        target: 'https://example.com/test',
        context: {
            task: 'test plugin integration',
            user: 'developer',
        },
        data: {
            message: 'Hello from frontend!'
        }
    }
});
console.log('Execution result:', result);
```

## 3. 在Agent中配置插件工具

### 在AgentManager.vue中

1. 打开Agent管理界面
2. 新建或编辑一个Agent
3. 在"可用工具"部分，展开"插件工具"折叠面板
4. 勾选 `Agent问候插件`
5. 保存Agent配置

此时Agent的配置中应包含：
```json
{
  "tools": {
    "allow": [
      "http_request",
      "plugin::test.hello_agent"
    ]
  }
}
```

## 4. 测试Agent调用插件

### 方法1：通过对话测试

与Agent对话，让它使用插件：

```
用户: "请使用Agent问候插件测试一下 https://example.com"

Agent: (会看到plugin::test.hello_agent在可用工具列表中)
      "好的，我将使用Agent问候插件进行测试..."
      
      [调用 plugin::test.hello_agent]
      
      "✅ 插件调用成功！发现以下信息：
       - 类型: info
       - 标题: Agent插件调用成功
       - 描述: 插件收到Agent调用！目标: https://example.com
       - 证据: {...}
       
       插件工作正常，可以接收Agent的调用并返回结果。"
```

### 方法2：检查工具过滤日志

查看后端日志，应该能看到：

```
[INFO] AgentPluginProvider discovered X tools
[INFO] Tool: plugin::test.hello_agent - 测试Agent调用插件的功能
[INFO] ReAct executor: 工具过滤配置 - 白名单: ["plugin::test.hello_agent", ...]
[INFO] ReAct executor: 构建工具信息，共 Y 个工具
```

## 5. 验证完整调用链路

### 检查点1: Provider注册

启动应用时，日志应显示：
```
[INFO] Agent plugin provider registered successfully
```

### 检查点2: 工具发现

Agent执行时，日志应显示：
```
[INFO] ReAct executor: 框架适配器提供了 X 个工具
[DEBUG] ReAct executor: 工具 'plugin::test.hello_agent' 在白名单中
```

### 检查点3: 工具调用

Agent决定使用插件时：
```
[INFO] Executing dynamic tool: plugin::test.hello_agent
[INFO] Plugin execution started: test.hello_agent
[INFO] Plugin execution completed successfully
```

### 检查点4: 结果返回

检查ToolExecutionResult：
```json
{
  "tool_name": "plugin::test.hello_agent",
  "success": true,
  "output": {
    "plugin_id": "test.hello_agent",
    "findings": [...],
    "count": 1
  }
}
```

## 6. 开发自己的Agent插件

### 插件接口约定

Agent调用插件时，会通过 `RequestContext.body` 传递参数：

```typescript
interface AgentPluginInput {
  context: object;  // Agent提供的分析上下文
  data: any;        // Agent提供的输入数据
  inputs: object;   // 完整的工具调用参数（包含target等）
}
```

### 插件示例模板

```typescript
export default {
  id: 'custom.my_analyzer',
  name: '我的分析器',
  category: 'agentTools',  // 必须是 agentTools
  version: '1.0.0',
  
  async onRequest(ctx: RequestContext): Promise<Finding[]> {
    // 1. 解析Agent传入的参数
    const bodyText = new TextDecoder().decode(ctx.body);
    const { context, data, inputs } = JSON.parse(bodyText);
    
    const target = inputs.target || ctx.url;
    
    // 2. 执行分析逻辑
    const findings: Finding[] = [];
    
    try {
      // 你的分析代码
      const analysisResult = await myAnalysisLogic(target, data);
      
      if (analysisResult.hasIssue) {
        findings.push({
          vuln_type: 'security_issue',
          severity: 'medium',
          title: '发现安全问题',
          description: analysisResult.description,
          evidence: analysisResult.evidence,
          url: target,
          timestamp: new Date().toISOString(),
        });
      }
    } catch (error) {
      // 错误也可以作为finding返回
      findings.push({
        vuln_type: 'error',
        severity: 'info',
        title: '插件执行错误',
        description: error.message,
        evidence: { error: String(error) },
        url: target,
        timestamp: new Date().toISOString(),
      });
    }
    
    return findings;
  }
}
```

## 7. 故障排查

### 问题1: 插件工具未出现在列表中

**检查**:
- 插件的 `category` 是否为 `"agentTools"`
- 插件是否已启用 (`enabled = true`)
- 后端日志是否显示 "Agent plugin provider registered successfully"

**解决**:
```javascript
// 检查插件列表
const tools = await window.__TAURI__.invoke('list_agent_plugin_tools');
console.log(tools);

// 如果为空，检查数据库
SELECT * FROM passive_plugin_registry WHERE category = 'agentTools';
```

### 问题2: Agent看不到插件工具

**检查**:
- Agent配置中是否选中了插件（`tools.allow` 包含 `plugin::xxx`）
- 工具系统是否正确初始化

**解决**:
```javascript
// 检查Agent配置
const agents = await window.__TAURI__.invoke('list_scenario_agents');
console.log(agents.find(a => a.id === 'your-agent-id').tools);
```

### 问题3: 插件执行失败

**检查**:
- 插件代码是否有语法错误
- 插件是否正确解析 RequestContext.body
- 插件是否返回了 Finding[] 数组

**解决**:
```javascript
// 直接测试插件执行
const result = await window.__TAURI__.invoke('test_execute_plugin_tool', {
    request: {
        plugin_id: 'your_plugin_id',
        target: 'test',
        data: {}
    }
});
console.log('Error:', result.error);
```

## 8. 高级用法

### 插件间协作

Agent可以组合多个插件工具：

```
用户: "分析这个URL的安全性: https://example.com"

Agent: 
1. 调用 plugin::url_analyzer (分析URL结构)
2. 调用 plugin::sqli_detector (检测SQL注入)
3. 调用 plugin::xss_detector (检测XSS)
4. 综合分析结果，生成报告
```

### 上下文传递

Agent可以在多次插件调用间传递上下文：

```typescript
// 第一次调用
call plugin::initial_scan({
  target: 'https://example.com',
  context: { task: 'initial scan' }
})

// 第二次调用，使用第一次的结果
call plugin::deep_analysis({
  target: 'https://example.com',
  context: { 
    task: 'deep analysis',
    previous_findings: [...] 
  }
})
```

## 总结

✅ **已完成**:
- AgentPluginProvider 实现
- 工具注册到全局系统
- 前端UI集成
- 测试命令提供

🎯 **下一步**:
1. 创建测试插件验证功能
2. 在真实Agent对话中测试
3. 开发更多实用的agentTools插件

🚀 **建议插件**:
- `agent.port_scanner` - 端口扫描分析
- `agent.subdomain_finder` - 子域名发现
- `agent.tech_detector` - 技术栈识别
- `agent.vulnerability_scanner` - 漏洞扫描
- `agent.report_generator` - 报告生成器
