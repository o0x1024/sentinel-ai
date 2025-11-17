# ReWOO消息显示优化

## 问题描述

ReWOO架构返回的消息以纯文本形式展示，包含了大量结构化信息（Planning、Execution、Solving三个阶段），但前端没有进行格式化处理，导致排版混乱、信息不清晰。

## 解决方案

### 1. 创建ReWOO专用消息显示组件

**文件**: `src/components/MessageParts/ReWOOStepDisplay.vue`

该组件支持三个阶段的结构化展示：

#### Planning阶段
- 显示整体规划思路
- 列出所有执行步骤（#E1, #E2, ...）
- 展示每个步骤的工具名称和描述

#### Execution阶段
- 按步骤展示工具执行过程
- 支持折叠/展开详情
- 显示思考过程、执行结果、错误信息
- 用不同颜色标识成功/失败/运行中状态

#### Solving阶段
- 展示最终答案（Markdown渲染）
- 显示执行元数据（耗时、步骤数等）

### 2. 创建消息解析工具

**文件**: `src/composables/useReWOOMessage.ts`

提供以下功能：

- `isReWOOMessage()`: 检测消息是否为ReWOO格式
- `parseReWOOMessage()`: 解析ReWOO消息的各个阶段
- `extractReWOOSummary()`: 提取消息摘要

解析逻辑：
1. 从chunks中识别stage标识（rewoo_planning、rewoo_execution、rewoo_solving）
2. 从content中解析Planning格式（Plan: ... #E1 = ...）
3. 从chunks和content中提取工具执行信息
4. 组合成结构化数据供组件使用

### 3. 集成到AIChat组件

**文件**: `src/components/AIChat.vue`

修改内容：
1. 导入ReWOO相关组件和工具
2. 添加ReWOO消息检测函数
3. 在消息渲染逻辑中优先检测ReWOO格式
4. 使用ReWOOStepDisplay组件渲染

渲染优先级：
```
ReWOO消息 > ReAct消息 > 普通Markdown消息
```

## 技术细节

### 消息块类型（ChunkType）

ReWOO使用以下chunk类型：
- `Thinking`: 思考过程
- `PlanInfo`: 计划信息
- `ToolResult`: 工具执行结果
- `Content`: 主要内容（Solver的最终答案）
- `Meta`: 元数据信息
- `Error`: 错误信息

### Stage标识

- `rewoo_planning`: Planning阶段
- `rewoo_execution`: Execution阶段
- `rewoo_solving`: Solving阶段

### 样式设计

- Planning: 黄色边框，灯泡图标
- Execution: 蓝色边框，齿轮图标，可折叠步骤
- Solving: 绿色边框，对勾图标

## 依赖变更

新增依赖：
```bash
npm install marked
```

用于Markdown渲染（Planning摘要和Solving最终答案）。

## 测试

编译测试通过：
```bash
npm run build
✓ built in 6.40s
```

## 使用效果

### 改进前
- 所有信息混在一起，难以阅读
- 无法区分不同阶段
- 工具执行结果格式混乱
- 缺少视觉层次

### 改进后
- 三个阶段清晰分离
- 每个阶段有独特的视觉标识
- 工具执行步骤可折叠展开
- Markdown格式化的最终答案
- 状态标识（成功/失败/运行中）
- 更好的信息层次和可读性

## 未来改进方向

1. 添加步骤间依赖关系的可视化
2. 支持实时更新执行进度
3. 添加步骤执行时间统计
4. 支持导出ReWOO执行报告
5. 添加步骤重试功能

