# Audit Gate CI Usage

This directory contains CI helpers for enforcing audit policy gate results.

## 1) Input Contract

`check-audit-gate.mjs` accepts either:

- Direct `AuditGateCiResult` JSON:

```json
{
  "conversation_id": "conv-123",
  "passed": false,
  "should_block": true,
  "reason": "Blocked by prod_strict policy: critical=0, high=1",
  "profile": "prod_strict",
  "source": "stored_policy_gate"
}
```

- Tauri command wrapper JSON:

```json
{
  "success": true,
  "data": {
    "conversation_id": "conv-123",
    "passed": true,
    "should_block": false,
    "reason": "Passed",
    "source": "stored_policy_gate"
  },
  "error": null
}
```

## 2) Local Check

```bash
npm run ci:audit-gate -- --input .artifacts/audit_gate_result.json
```

Exit codes:

- `0`: pass
- `1`: block
- `2`: invalid input/usage

## 3) End-to-End CI Pattern

1. In a previous step, generate `AuditGateCiResult` via backend command `get_audit_gate_for_ci`.
2. Save JSON to `.artifacts/audit_gate_result.json`.
3. Run gate check script.

### GitHub Actions Example

```yaml
- name: Check audit gate
  run: npm run ci:audit-gate -- --input .artifacts/audit_gate_result.json
```

### Fail-open Mode (Optional)

If you want missing gate to pass:

```bash
npm run ci:audit-gate -- --input .artifacts/audit_gate_result.json --fail-on-missing false
```
