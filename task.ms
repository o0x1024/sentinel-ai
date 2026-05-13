# 资产清单百万级性能改造任务

## 目标

- 资产清单列表查询不再依赖深 `OFFSET`。
- 列表接口不再同步执行全量 `COUNT(*)`。
- 导出不再把全量资产拉到前端内存。
- 前端分页、统计、导出交互适配大数据场景。

## 实施项

1. 后端新增 cursor 分页请求/响应结构，基于 `(last_seen_at, id)` 做 keyset 分页。
2. `surface_list_inventory` 改为只返回当前窗口、`has_next`、`next_cursor`，不再同步统计总数。
3. 保留独立 `surface_count_assets` 作为异步统计接口，前端只在筛选变化时刷新 total。
4. 运行记录 `surface_list_discovery_runs` 前端强制分页，不再 `limit: null` 拉全量。
5. 新增后端资产导出命令，按 cursor 分批查询并写入目标文件。
6. 前端导出改为选择保存路径后调用后端导出命令，不在前端拼装百万级 CSV/JSON。
7. 前端分页改为 cursor 栈，移除依赖深页随机跳转的交互路径。
8. 运行测试覆盖 cursor SQL、导出过滤语义和前端构建相关类型。

## 不做

- 不做兼容旧的深分页跳页方案。
- 不做前端全量加载 fallback。
- 不做 `LIKE '%keyword%'` 搜索的最终搜索引擎改造，本轮先移除列表路径中最重的同步 COUNT 和深 OFFSET。
