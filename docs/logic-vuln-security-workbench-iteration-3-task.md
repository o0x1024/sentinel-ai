# 安全工作台迭代三任务文档

本轮任务从“调查闭环”继续向“水平越权实战辅助”推进，但仍然保持只读优先，不直接发起主动测试。

关联文档：

- [logic-vuln-ultimate-solution.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-ultimate-solution.md)
- [logic-vuln-security-workbench-implementation.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-implementation.md)
- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-2-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-2-task.md)

---

## 1. 本次目标

让安全工作台不再只是“看证据和记结论”，而是开始具备对白帽更直接有用的对象边界分析能力。

本轮聚焦三件事：

1. 从现有 evidence 中提取对象引用候选
2. 推断最小行为链
3. 基于对象池生成水平越权测试建议

---

## 2. 本次范围

### 包含

1. 新增对象边界分析 support 模块
2. 支持从 path / query / request body / response body 抽取对象候选
3. 支持按对象角色归并对象池
4. 支持推断列表、详情、修改、导出、敏感动作等行为链节点
5. 支持生成只读优先的水平越权测试建议
6. 在工作台详情新增“对象边界分析”面板

### 不包含

1. 自动发起 replay
2. 行为埋点级真实前端行为链
3. 图形化对象关系图
4. 自动执行批量替换

---

## 3. 交付物

### 前端 support

1. `securityWorkbenchObjectAnalysis.ts`
2. 对象候选抽取
3. 对象池归并
4. 行为链推断
5. 水平越权测试建议生成

### 前端面板

1. `WorkbenchObjectAnalysisPanel.vue`
2. 工作台详情新增 `对象边界分析` tab

---

## 4. 验收标准

1. 工作台详情中可以看到对象边界分析 tab
2. 能从 evidence 中提取出稳定的对象池候选
3. 能看到基础行为链推断结果
4. 能看到“替换哪个字段、用哪些候选值试”的测试建议
5. 本轮仍然不自动发请求，只提供建议

---

## 5. 当前实现结论

本轮完成后，安全工作台已经具备：

`台账 -> 案件 -> 证据 -> 对象池 -> 行为链 -> 水平越权建议`

也就是说，它已经开始从“漏洞调查台”向“白帽实战辅助台”过渡，但还没有进入自动验证执行阶段。

下一轮优先级建议：

1. 把建议变成可点击的 replay plan
2. 引入对象关系图和父子错配分析
3. 再决定是否接入真正的主动验证执行
