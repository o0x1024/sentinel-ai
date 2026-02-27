#!/usr/bin/env bash
set -euo pipefail

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required but not installed." >&2
  exit 1
fi

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <sqlite_db_path> [output_dir]" >&2
  exit 1
fi

DB_PATH="$1"
OUT_DIR="${2:-.artifacts}"

if [[ ! -f "$DB_PATH" ]]; then
  echo "Database file not found: $DB_PATH" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
TS="$(date +"%Y%m%d-%H%M%S")"

REPORT_MD="$OUT_DIR/agent-team-v2-upgrade-rehearsal-$TS.md"
LEGACY_CSV="$OUT_DIR/agent-team-v2-legacy-templates-$TS.csv"
FAILED_CSV="$OUT_DIR/agent-team-v2-upgrade-failed-$TS.csv"
BLOCKERS_CSV="$OUT_DIR/agent-team-v2-blockers-$TS.csv"
DUP_MEMBER_CSV="$OUT_DIR/agent-team-v2-duplicate-members-$TS.csv"

has_templates_table="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_team_templates';")"
if [[ "$has_templates_table" != "1" ]]; then
  echo "agent_team_templates table not found in: $DB_PATH" >&2
  exit 2
fi

has_members_table="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_team_template_members';")"
if [[ "$has_members_table" != "1" ]]; then
  echo "agent_team_template_members table not found in: $DB_PATH" >&2
  exit 2
fi

has_schema_version_col="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM pragma_table_info('agent_team_templates') WHERE name='schema_version';")"
has_upgrade_failed_col="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM pragma_table_info('agent_team_templates') WHERE name='upgrade_failed';")"
has_upgrade_error_col="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM pragma_table_info('agent_team_templates') WHERE name='upgrade_error';")"

schema_expr="1"
if [[ "$has_schema_version_col" == "1" ]]; then
  schema_expr="COALESCE(t.schema_version, 1)"
fi

upgrade_failed_expr="0"
if [[ "$has_upgrade_failed_col" == "1" ]]; then
  upgrade_failed_expr="COALESCE(t.upgrade_failed, 0)"
fi

upgrade_error_expr="''"
if [[ "$has_upgrade_error_col" == "1" ]]; then
  upgrade_error_expr="COALESCE(t.upgrade_error, '')"
fi

legacy_query="
SELECT
  t.id,
  t.name,
  $schema_expr AS schema_version,
  COUNT(m.id) AS member_count
FROM agent_team_templates t
LEFT JOIN agent_team_template_members m ON m.template_id = t.id
GROUP BY t.id, t.name
HAVING ($schema_expr < 2)
ORDER BY t.updated_at DESC, t.created_at DESC
"

failed_query="
SELECT
  t.id,
  t.name,
  $schema_expr AS schema_version,
  $upgrade_failed_expr AS upgrade_failed,
  $upgrade_error_expr AS upgrade_error
FROM agent_team_templates t
WHERE ($upgrade_failed_expr = 1)
ORDER BY t.updated_at DESC, t.created_at DESC
"

blockers_query="
SELECT
  t.id,
  t.name,
  $schema_expr AS schema_version,
  COUNT(m.id) AS member_count,
  CASE
    WHEN COUNT(m.id) = 0 THEN 'missing_members'
    ELSE 'unknown'
  END AS blocker_reason
FROM agent_team_templates t
LEFT JOIN agent_team_template_members m ON m.template_id = t.id
GROUP BY t.id, t.name
HAVING ($schema_expr < 2) AND COUNT(m.id) = 0
ORDER BY t.updated_at DESC, t.created_at DESC
"

dup_member_query="
SELECT
  t.id AS template_id,
  t.name AS template_name,
  LOWER(TRIM(m.name)) AS normalized_member_name,
  COUNT(*) AS duplicate_count
FROM agent_team_templates t
JOIN agent_team_template_members m ON m.template_id = t.id
GROUP BY t.id, t.name, LOWER(TRIM(m.name))
HAVING COUNT(*) > 1
ORDER BY duplicate_count DESC, template_name ASC
"

sqlite3 "$DB_PATH" -header -csv "$legacy_query" > "$LEGACY_CSV"
sqlite3 "$DB_PATH" -header -csv "$failed_query" > "$FAILED_CSV"
sqlite3 "$DB_PATH" -header -csv "$blockers_query" > "$BLOCKERS_CSV"
sqlite3 "$DB_PATH" -header -csv "$dup_member_query" > "$DUP_MEMBER_CSV"

legacy_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM (${legacy_query});")"
upgrade_failed_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM (${failed_query});")"
blocker_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM (${blockers_query});")"
dup_member_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM (${dup_member_query});")"

status="PASS"
if [[ "$blocker_count" -gt 0 || "$upgrade_failed_count" -gt 0 ]]; then
  status="FAIL"
elif [[ "$legacy_count" -gt 0 || "$dup_member_count" -gt 0 ]]; then
  status="WARN"
fi

{
  echo "# Agent Team V2 升级预演报告"
  echo
  echo "- 时间: $(date +"%Y-%m-%d %H:%M:%S %z")"
  echo "- 数据库: \`$DB_PATH\`"
  echo "- 结论: **$status**"
  echo
  echo "## 概览"
  echo "- schema_version < 2 模板数: $legacy_count"
  echo "- upgrade_failed 模板数: $upgrade_failed_count"
  echo "- 明确阻断（无成员）模板数: $blocker_count"
  echo "- 成员重名风险模板数: $dup_member_count"
  echo
  echo "## 产物文件"
  echo "- Legacy 模板列表: \`$LEGACY_CSV\`"
  echo "- 升级失败列表: \`$FAILED_CSV\`"
  echo "- 阻断模板列表: \`$BLOCKERS_CSV\`"
  echo "- 成员重名风险列表: \`$DUP_MEMBER_CSV\`"
  echo
  echo "## 建议"
  echo "1. 先处理阻断模板（无成员），再执行批量升级。"
  echo "2. 对成员重名模板做人工确认，避免 Agent 映射歧义。"
  echo "3. 对已有 upgrade_failed 模板先修复再发布。"
} > "$REPORT_MD"

echo "Upgrade rehearsal report generated:"
echo "  $REPORT_MD"

if [[ "$status" == "FAIL" ]]; then
  exit 3
fi

exit 0
