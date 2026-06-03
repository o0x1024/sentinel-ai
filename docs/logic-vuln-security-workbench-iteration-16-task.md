# 安全工作台迭代 16 任务单

本文档记录把工作台深链接从“能打开和定位”推进到“可直接复制分享”的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-15-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-15-task.md)

---

## 1. 本轮目标

上一轮已经支持：

- URL 深链接
- 自动定位 tab
- 自动滚动到目标卡片

但用户仍然需要手动复制地址栏，才能把当前定位点分享出去。

因此本轮目标是：

`为工作台增加“复制当前定位链接”入口。`

---

## 2. 本轮实现

### 2.1 页头复制入口

当当前处于某个案件详情时，工作台页头现在会显示：

- `复制定位链接`
- `返回案件列表`

两个入口。

### 2.2 复制内容

复制内容基于当前真实路由状态生成，包含：

- `caseId`
- `workbenchTab`
- `evidenceId`
- `draftId`
- `runId`

也就是说，复制出去的不是“案件首页链接”，而是当前精确定位到的案件内上下文链接。

### 2.3 生成方式

链接生成基于当前 router 状态解析，统一使用工作台已有的深链接 query，不新造额外状态字段。

---

## 3. 当前收益

这一轮完成后，工作台的案件定位能力真正具备了分享价值：

1. 时间线或验证过程中的当前位置可以直接复制
2. 不需要再手动整理 URL
3. 分享出去的链接仍然能落到具体 tab 和目标卡片

---

## 4. 当前限制

这一轮仍然保持轻量：

1. 只支持复制当前位置，不支持生成短链接
2. 还没有在时间线节点旁边单独提供复制入口
3. 还没有支持“复制纯案件链接”和“复制精确定位链接”的双模式

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 把案件列表筛选和分页状态同步到 URL
2. 支持复制“纯案件链接”与“当前定位链接”
3. 在时间线节点级别增加快速复制定位链接入口
