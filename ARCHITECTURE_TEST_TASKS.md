# 架构测试任务指南

## ⚠️ ReWOO架构状态

**重要**: ReWOO架构当前已被**禁用（DISABLED）**

### 禁用原因
```rust
// src-tauri/src/engines/rewoo/engine_adapter.rs
description: "DISABLED - ReWOO engine needs Rig refactor"

pub async fn new_with_dependencies(...) -> Result<Self> {
    Err(anyhow::anyhow!("ReWOO engine disabled - needs complete Rig refactor"))
}
```

### 状态
- ❌ 无法创建引擎实例
- ❌ 无法执行任务
- ⏸️ 等待Rig框架重构完成
- 📝 代码结构完整，但接口已禁用

**建议**: 暂时跳过ReWOO测试，等待后续重构完成

---

## ✅ LLM Compiler架构测试

### 架构特点
- **核心能力**: DAG（有向无环图）并行执行
- **最大并发**: 10个任务
- **适合场景**: 
  - 复杂多步骤任务
  - 高并发处理
  - 大规模批量操作
  - 任务间有依赖关系

### 推荐测试任务

#### 任务1: 批量端口扫描（⭐⭐⭐⭐⭐ 推荐）
```
扫描以下3个目标的常见端口状态：
- 172.28.32.97
- 172.28.51.248
- 172.28.63.178


检查每个目标的 80, 443, 22, 3306 端口是否开放
```

**为什么适合LLM Compiler**:
- ✅ 5个目标可以并行扫描
- ✅ 每个目标的多个端口可以并行检测
- ✅ 任务间无依赖，完美利用并行能力
- ✅ 可以测试DAG调度和并发执行

#### 任务2: 多目标信息收集（⭐⭐⭐⭐）
```
同时收集以下3个网站的基本信息：
1. example.com - 获取首页标题和meta信息
2. github.com - 检查响应时间和状态码
3. google.com - 分析HTTP头信息

要求：
- 并行执行所有请求
- 总结各网站的技术栈
- 对比响应时间
```

**为什么适合LLM Compiler**:
- ✅ 3个独立的网站信息收集任务
- ✅ 可以完全并行执行
- ✅ 最后需要汇总（测试Joiner组件）

#### 任务3: 批量API测试（⭐⭐⭐⭐）
```
测试RESTful API的多个端点：
- GET /api/users - 获取用户列表
- GET /api/posts - 获取文章列表  
- GET /api/comments - 获取评论列表
- GET /api/tags - 获取标签列表

目标: https://jsonplaceholder.typicode.com

要求：
- 并行请求所有端点
- 检查响应时间
- 验证数据结构
- 统计每个端点的数据量
```

**为什么适合LLM Compiler**:
- ✅ 4个独立API端点
- ✅ 完全并行执行
- ✅ 测试高并发能力
- ✅ 真实可用的测试API

#### 任务4: 并行漏洞扫描（⭐⭐⭐⭐⭐ 最推荐）
```
对测试靶场执行并行漏洞扫描：
目标: http://testphp.vulnweb.com

同时测试：
1. SQL注入 - 测试所有表单和参数
2. XSS漏洞 - 测试所有输入框
3. 敏感文件 - 扫描常见敏感路径
4. 目录遍历 - 测试路径遍历漏洞

要求：
- 4种漏洞类型并行扫描
- 实时报告发现的问题
- 最后汇总所有漏洞
```

**为什么最适合**:
- ✅ 符合安全测试场景
- ✅ 4个检测任务完全独立
- ✅ 可以测试真实被动扫描集成
- ✅ 展示LLM Compiler在安全领域的优势

#### 任务5: 复杂依赖任务（⭐⭐⭐ 进阶）
```
执行有依赖关系的任务链：

Step 1 (并行):
  - Task A: 获取目标网站的子域名列表
  - Task B: 获取目标网站的IP信息

Step 2 (依赖Step 1):
  - Task C: 对每个子域名进行端口扫描（依赖Task A）
  - Task D: 对IP进行反查域名（依赖Task B）

Step 3 (依赖Step 2):
  - Task E: 汇总所有发现，生成完整报告

目标: example.com
```

**为什么是进阶**:
- ✅ 测试DAG依赖调度
- ✅ 混合并行和串行执行
- ✅ 考验TaskFetchingUnit能力
- ⚠️ 需要多个工具配合

---

## 🎯 快速测试步骤

### 方式1: 使用AI聊天（推荐）

1. 打开应用
2. 在AI聊天中输入以下内容之一：

**简单测试**:
```
使用LLM Compiler架构，测试以下API端点的响应时间：
- https://jsonplaceholder.typicode.com/users
- https://jsonplaceholder.typicode.com/posts
- https://jsonplaceholder.typicode.com/comments

要求并行执行并汇总结果
```

**安全测试**:
```
使用LLM Compiler架构，并行扫描 testphp.vulnweb.com：
- SQL注入检测
- XSS漏洞检测
- 敏感文件扫描

实时显示每个任务的进度
```

### 方式2: 指定架构参数

如果需要明确指定架构，可以在请求中说明：
```
架构: LLM Compiler
任务: [你的任务描述]
```

---

## 📊 预期效果

### LLM Compiler执行流程
```
用户输入任务
  ↓
Planner: 生成DAG任务图
  ├─ Task 1 (pending)
  ├─ Task 2 (pending)
  ├─ Task 3 (pending)
  └─ Task 4 (pending)
  ↓
TaskFetchingUnit: 调度可执行任务
  ↓
ParallelExecutorPool: 并行执行
  ├─ Task 1 (executing) ───┐
  ├─ Task 2 (executing) ───┤
  ├─ Task 3 (executing) ───┼─→ 同时执行
  └─ Task 4 (executing) ───┘
  ↓
IntelligentJoiner: 汇总结果
  ↓
返回最终答案
```

### 观察点
- ✅ 任务是否并行执行
- ✅ 总执行时间是否短于串行
- ✅ DAG调度是否正确
- ✅ 结果汇总是否准确
- ⚠️ 取消功能（部分支持，可取消pending任务）

---

## ⚠️ 注意事项

### LLM Compiler限制
1. **取消机制**: 只能取消pending任务，无法停止正在执行的任务
2. **最大并发**: 默认10，超过会排队
3. **超时时间**: 默认300秒，超时会失败

### 测试建议
1. 先用简单任务测试基本功能
2. 再用复杂任务测试并行能力
3. 尝试点击停止按钮测试取消功能
4. 观察日志了解执行过程

---

## 🔄 替代方案（如果想测试类似ReWOO的场景）

由于ReWOO不可用，可以用以下方案测试类似功能：

### Plan-and-Execute架构
```
使用Plan-and-Execute架构执行多步骤任务：

Step 1: 收集目标信息
Step 2: 分析技术栈
Step 3: 生成测试策略
Step 4: 执行安全测试
Step 5: 生成测试报告

目标: example.com
```

**特点**:
- 有序执行（非并行）
- 支持重新规划
- 支持取消（已完全实现）

---

## 📝 总结

| 架构 | 状态 | 推荐任务 | 取消支持 |
|------|------|---------|---------|
| **LLM Compiler** | ✅ 可用 | 并行漏洞扫描 | ⚠️ 部分 |
| **ReWOO** | ❌ 禁用 | - | - |
| **ReAct** | ✅ 可用 | 探索性渗透测试 | ✅ 完全 |
| **Plan-and-Execute** | ✅ 可用 | 多步骤有序任务 | ✅ 完全 |

**最终建议**:
1. ✅ 测试LLM Compiler - 使用"并行漏洞扫描"任务
2. ⏸️ 跳过ReWOO - 等待Rig重构完成
3. ✅ 如需对比，可用Plan-and-Execute或ReAct

---

**创建时间**: 2025-11-13  
**应用状态**: ✅ 运行在 localhost:1420  
**准备测试**: ✅ 可以开始

