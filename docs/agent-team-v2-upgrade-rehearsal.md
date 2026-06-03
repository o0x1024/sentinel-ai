# Agent Team V2 升级预演

## 目的
- 发布前离线扫描现有模板，提前识别 V2 强制升级的阻断项与风险项。
- 该预演脚本**只读**数据库，不会修改模板数据。

## 使用方式
```bash
scripts/agent-team-v2-upgrade-rehearsal.sh <sqlite_db_path> [output_dir]
```

示例：
```bash
scripts/agent-team-v2-upgrade-rehearsal.sh ~/Library/Application\\ Support/sentinel/sentinel.db .artifacts
```

## 输出内容
- `agent-team-v2-upgrade-rehearsal-<ts>.md`：总览报告与建议。
- `agent-team-v2-legacy-templates-<ts>.csv`：`schema_version < 2` 模板列表。
- `agent-team-v2-upgrade-failed-<ts>.csv`：历史 `upgrade_failed` 模板列表。
- `agent-team-v2-blockers-<ts>.csv`：明确阻断模板（例如缺失成员）。
- `agent-team-v2-duplicate-members-<ts>.csv`：成员重名风险列表。

## 退出码
- `0`：预演通过（PASS/WARN）。
- `3`：预演失败（存在阻断模板或已有升级失败模板）。
- `1/2`：脚本参数或数据库结构异常。
