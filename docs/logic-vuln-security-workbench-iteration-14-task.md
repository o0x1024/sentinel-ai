# 安全工作台迭代 14 任务单

本文档记录把案件时间线跳转进一步推进到 URL 深链接的这一轮实现。

关联文档：

- [logic-vuln-security-workbench-mvp-tasklist.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-mvp-tasklist.md)
- [logic-vuln-security-workbench-iteration-13-task.md](/Users/like/code/sentinel/sentinel-ai/docs/logic-vuln-security-workbench-iteration-13-task.md)

---

## 1. 本轮目标

上一轮已经完成：

- 时间线搜索
- 从时间线节点跳到证据、草案、执行结果
- 目标面板高亮

但这些导航还是停留在页面内部状态，不能通过 URL 直接复现或分享。

因此本轮目标是：

`把工作台案件详情的定位状态同步到 URL，支持深链接。`

---

## 2. 本轮实现

### 2.1 URL 深链接参数

工作台案件详情现在会把关键定位状态同步到 URL query，包括：

- `workbenchTab`
- `evidenceId`
- `draftId`
- `runId`

这样一个工作台链接不再只是“打开某个案件”，而是可以直接定位到案件内的具体上下文。

### 2.2 页面级统一路由管理

深链接状态统一收口在工作台 page 层管理：

- page 负责解析 URL
- case detail 只消费当前状态并向上发事件
- 具体面板不直接操作路由

这样可以避免路由状态散落到多个子组件里。

### 2.3 时间线跳转与 URL 同步打通

现在从时间线点击：

- 查看证据
- 查看草案
- 查看执行结果

不仅会切换到对应 tab，也会同步更新 URL，因此刷新页面或直接分享链接后，仍然能回到同一个定位点。

### 2.4 手工切换 tab 也会同步到 URL

案件详情手工切换 tab 时，当前 tab 也会写回 URL，不再只是本地状态。

---

## 3. 当前收益

这一轮完成后，安全工作台更接近真正可复盘、可分享的案件工作区：

1. 可以直接分享案件内部定位链接
2. 刷新后不会丢失当前 tab
3. 时间线导航和 URL 状态一致

---

## 4. 当前限制

这一轮仍然保持轻量：

1. 还没有把列表筛选同步到 URL
2. 还没有支持从外部 finding 链接直接带入工作台定位参数
3. 目标高亮仍然只在当前页面实例内体现，不做滚动锚点

---

## 5. 下一轮建议

如果继续推进，下一轮最值得做的是：

1. 给定位目标补滚动锚点
2. 把列表筛选也同步到 URL
3. 支持从漏洞中心一键生成“带定位参数”的工作台链接
