---
name: code-audit
description: "Structured code audit workflow using Code Property Graph (CPG) and V2 analysis tools for comprehensive LLM-led security analysis. Guides the AI through a multi-phase audit process: reconnaissance, attack surface analysis, deep data flow tracing, and reporting."
when_to_use: "When performing code security audit, code review, vulnerability analysis, or security assessment tasks"
---

# CPG-Powered Code Audit Workflow (V2)

## Overview
This workflow guides you through a structured, multi-phase code security audit using the Code Property Graph (CPG) engine and V2 analysis tools. The LLM acts as the audit brain — understanding business logic, generating hypotheses, and using tools to verify them.

### Available V2 Tools
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `build_cpg` | Build Code Property Graph | Once at start |
| `get_attack_surface` | Enumerate all endpoints + auth status | Reconnaissance |
| `smart_file_summary` | High-density file skeleton + security signals | Before reading a file |
| `get_function_detail` | Rich function signatures + security context | Understanding interfaces |
| `trace_data_flow` | Follow data through call chains to sinks | Verifying injection/taint |
| `query_cpg` | Graph queries (functions, classes, callers) | Architecture understanding |
| `cpg_security_scan` | All 15 rules baseline scan | Catching known patterns |
| `cpg_taint_analysis` | Targeted taint for specific rules | Deep analysis |
| `read_file` | Read actual source code | Confirming findings |
| `code_search` | ripgrep pattern search | Finding specific patterns |
| `dependency_audit` | Dependency vulnerability scan | Supply chain risk |

## Phase 1: Reconnaissance (1-2 tool calls)

### Step 1.1: Build the Code Property Graph
```
build_cpg({ path: "<project_root>" })
```
This parses all source files and builds a structural graph of the codebase.

### Step 1.2: Map Attack Surface (NEW — use this instead of project_overview)
```
get_attack_surface({ path: "<project_root>" })
```
This returns:
- **All HTTP endpoints** with: route, method, handler function, auth status
- **Unprotected endpoint count** — prioritize these
- **Input-receiving functions** — potential injection targets
- **Database access functions** — potential SQL injection sinks
- **Risk indicators** per endpoint

> **Key Decision Point**: From the attack surface output, identify:
> 1. Endpoints with `auth_status: "unknown"` or `"unprotected"` → audit first
> 2. Endpoints with risk indicators like "Command execution" → critical priority
> 3. Endpoints with multiple risk indicators → highest priority

## Phase 2: Business Logic & Architecture Understanding

### Step 2.1: Understand Key Controllers (use smart_file_summary)
For each critical file identified in Phase 1:
```
smart_file_summary({ path: "<handler_file>", focus: "security" })
```
This gives you:
- **File skeleton** — all functions with signatures and parameters (no need to read 1000+ lines)
- **Security signals** — database queries, external input, command exec locations
- **Hotspots** — most dangerous code regions worth reading in detail
- **Exposed endpoints** — which routes this file handles

### Step 2.2: Deep Function Analysis (use get_function_detail)
For suspicious functions found in Phase 1 or Phase 2.1:
```
get_function_detail({
  path: "<project_root>",
  function_name: "UserService.*"
})
```
Returns per function:
- Full signature with parameters and types
- **Security context**: auth checks, DB ops, command exec, external input
- Callers and callees
- Risk indicators

> **Pattern to look for**: Functions where `accepts_external_input: true` AND `has_db_operation: true` AND `has_auth_check: false`

### Step 2.3: Build Role-Permission Mental Model
```
get_function_detail({ path: "<project_root>", function_name: "*auth*" })
get_function_detail({ path: "<project_root>", function_name: "*admin*" })
query_cpg({ path: "<project_root>", query: { type: "search", query: "role" } })
```
Build a mental model:
- Who can access what?
- Are there missing permission checks on critical actions?
- Can standard users access admin endpoints (Broken Access Control)?
- Can users modify another user's data (IDOR)?

## Phase 3: Baseline Security Scan

### Step 3.1: Run Full Security Scan
```
cpg_security_scan({ path: "<project_root>" })
```
This runs all 15 vulnerability rules:

**Injection & Data Flow:**
- SQL Injection (CWE-89), XSS (CWE-79), Command Injection (CWE-78)
- Path Traversal (CWE-22), SSRF (CWE-918), Insecure Deserialization (CWE-502)
- LDAP Injection (CWE-90), XXE (CWE-611), Open Redirect (CWE-601), Log Injection (CWE-117)

**Credentials & Crypto:**
- Hardcoded Secrets (CWE-798), Cryptographic Misuse (CWE-327), Insecure Randomness (CWE-330)

**Access Control & Config:**
- Auth Bypass (CWE-862), Security Misconfiguration (CWE-16)

## Phase 4: Deep Analysis — Data Flow Tracing (V2 Core)

For each finding from Phase 3 or suspicious function from Phase 2:

### Step 4.1: Trace Data Flow (NEW — the core V2 analysis tool)
```
trace_data_flow({
  path: "<project_root>",
  from: "<suspicious_function>",
  direction: "forward",
  max_depth: 8
})
```
This traces data through the actual call graph:
- **Forward**: "Where does data from this function flow?" → finds sinks
- **Backward**: "Where does data in this function come from?" → finds sources

> **When to use forward vs backward:**
> - Forward: Starting from a user-input handler, trace to see if it reaches a dangerous sink
> - Backward: Starting from a dangerous sink (e.g., `db.query`), trace back to see if user input reaches it

### Step 4.2: Call Chain Investigation
```
query_cpg({
  path: "<project_root>",
  query: { type: "callers_of", function_name: "<vulnerable_function>" }
})
```
Check if other callers also reach the dangerous sink.

### Step 4.3: Read Source Code (targeted)
Only after identifying specific hotspots:
```
read_file({ path: "<file_path>", offset: X, limit: 50 })
```
Read the actual source code to confirm the vulnerability.

### Step 4.4: Confirm or Dismiss
If confirmed as a real vulnerability:
```
audit_finding_upsert({
  conversation_id: "<conversation_id>",
  findings: [{
    id: "sqli-userservice-save",
    title: "SQL Injection in UserService.save()",
    severity: "critical",
    lifecycle_stage: "candidate",
    verification_status: "pending",
    cwe: "CWE-89",
    files: ["src/services/UserService.java"],
    description: "User input from req.body flows into db.query() without parameterization",
    fix: "Use parameterized queries or prepared statements",
    confidence: 0.95,
    evidence: ["src/services/UserService.java:45 db.query(sql + req.body.name)"],
    required_evidence: ["Parameterized query migration diff", "Regression test proving sink blocked"],
    provenance: { source: "audit_agent", stage: "initial_triage" }
  }]
})
```

After judge/verifier consensus, transition lifecycle:
```
transition_agent_audit_finding_lifecycle({
  request: {
    finding_id: "<record_uuid>",
    lifecycle_stage: "confirmed",
    verification_status: "passed",
    judge: { verdict: "confirmed", confidence: 0.91 },
    verifier: { checks: ["sink_reachable", "exploitability_review"] },
    provenance: { source: "judge_subagent", reason: "evidence_sufficient" }
  }
})
```

## Phase 5: Cross-Cutting Analysis

### Step 5.1: Dependency Audit
```
dependency_audit({ path: "<project_root>", run_scanners: true })
```

### Step 5.2: Cryptographic Practices
```
cpg_taint_analysis({
  path: "<project_root>",
  rules: ["crypto_misuse", "insecure_random"],
  max_depth: 8
})
```

### Step 5.3: Auth & Authorization Review
```
cpg_taint_analysis({
  path: "<project_root>",
  rules: ["auth_bypass"],
  max_depth: 10
})
```
Cross-reference with the attack surface from Phase 1 to find endpoints without proper auth.

### Step 5.4: Configuration Security
```
cpg_taint_analysis({
  path: "<project_root>",
  rules: ["config_security"],
  max_depth: 5
})
```

## Phase 6: Tenth Man Review & Reporting

### Step 6.1: Challenge Findings
```
tenth_man_review({
  topic: "Code Audit Findings Review",
  findings: "<summary of all findings>"
})
```

### Step 6.2: Generate Report
```
audit_report({
  title: "Security Audit Report",
  format: "markdown",
  findings: [<all confirmed findings>],
  target: "<project_name>",
  auditor: "AI Security Auditor"
})
```

## Key Principles

1. **Attack surface first**: Use `get_attack_surface` before diving into code — know what to audit
2. **Summary before source**: Use `smart_file_summary` before `read_file` — save 80% context
3. **Function detail before code read**: Use `get_function_detail` to understand interfaces
4. **Trace data, don't grep**: Use `trace_data_flow` to follow actual call paths, not string patterns
5. **Understand business logic**: Build a role-permission model before looking for injection vulnerabilities
6. **Check the sanitizers**: A sanitized path is not necessarily safe — verify effectiveness
7. **Cross-file awareness**: Use `callers_of` / `callees_of` / `trace_data_flow(backward)` for full context
8. **Document everything**: Use `audit_finding_upsert` for confirmed vulns
9. **Think beyond injection**: Always check crypto, auth/authz, and configuration security
10. **Find logic flaws**: Exploit the LLM's intent-understanding to find IDOR, Race Conditions, and Broken Access Control
