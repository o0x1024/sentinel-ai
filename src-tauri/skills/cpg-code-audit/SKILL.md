---
name: cpg-code-audit
description: "Structured code audit workflow using Code Property Graph (CPG) for comprehensive security analysis. Guides the AI through a multi-phase audit process: reconnaissance, baseline scanning, deep analysis, and reporting."
when_to_use: "When performing code security audit, code review, vulnerability analysis, or security assessment tasks"
---

# CPG-Powered Code Audit Workflow

## Overview
This workflow guides you through a structured, multi-phase code security audit using the Code Property Graph (CPG) engine. The CPG provides AST-level code understanding across 11 languages.

## Phase 1: Reconnaissance (Auto / First Turn)

### Step 1.1: Build the Code Property Graph
```
build_cpg({ path: "<project_root>" })
```
This parses all source files and builds a structural graph of the codebase.

### Step 1.2: Project Overview
```
query_cpg({ path: "<project_root>", query: { type: "summary" } })
query_cpg({ path: "<project_root>", query: { type: "files", limit: 30 } })
```
Understand the project structure, languages used, and file distribution.

### Step 1.3: Identify Entry Points and Critical Functions
```
query_cpg({ path: "<project_root>", query: { type: "functions", limit: 50 } })
query_cpg({ path: "<project_root>", query: { type: "call_edges", limit: 50 } })
```
Identify HTTP endpoints, public APIs, and high fan-in functions (most-called = most critical).

## Phase 2: Baseline Security Scan (Automated)

### Step 2.1: Run Full Security Scan
```
cpg_security_scan({ path: "<project_root>" })
```
This runs all 11 vulnerability rules against the CPG:
- SQL Injection (CWE-89)
- XSS (CWE-79)
- Command Injection (CWE-78)
- Path Traversal (CWE-22)
- SSRF (CWE-918)
- Insecure Deserialization (CWE-502)
- LDAP Injection (CWE-90)
- XXE (CWE-611)
- Open Redirect (CWE-601)
- Log Injection (CWE-117)
- Hardcoded Secrets (CWE-798)

### Step 2.2: Create Audit Plan (Todos)
Based on the scan results, create a prioritized audit plan:
```
todos({
  action: "create",
  description: "Review SQL Injection finding in UserDao.java:45",
  priority: 1
})
```
Priority order:
1. Critical severity, unsanitized findings
2. High severity, unsanitized findings
3. Medium severity findings
4. Pattern-based findings (hardcoded secrets)
5. Sanitized findings (verify sanitizer effectiveness)

## Phase 3: Deep Analysis (AI-Guided)

For each high-priority finding from Phase 2:

### Step 3.1: Targeted Taint Analysis
```
cpg_taint_analysis({
  path: "<project_root>",
  rules: ["sql_injection"],
  max_depth: 10
})
```
Get precise source→sink trace paths for the specific vulnerability class.

### Step 3.2: Call Chain Investigation
```
query_cpg({
  path: "<project_root>",
  query: { type: "callers_of", function_name: "<vulnerable_function>" }
})
```
Check if other callers also reach the dangerous sink.

### Step 3.3: Read Source Code
```
read_file({ path: "<file_path>", start_line: X, end_line: Y })
```
Read the actual source code to confirm the vulnerability.

### Step 3.4: Confirm or Dismiss
If confirmed as a real vulnerability:
```
audit_finding_upsert({
  findings: [{
    title: "SQL Injection in UserService.save()",
    severity: "critical",
    cwe: "CWE-89",
    file: "src/services/UserService.java",
    line: 45,
    description: "User input from req.body flows into db.query() without parameterization",
    recommendation: "Use parameterized queries or prepared statements",
    confidence: 0.95
  }]
})
```

### Step 3.5: Update Coverage
```
audit_coverage({
  action: "mark_audited",
  item: "src/services/UserService.java"
})
todos({ action: "done", id: "<todo_id>" })
```

## Phase 4: Cross-Cutting Analysis

After individual findings, look for systemic issues:

### Step 4.1: Dependency Audit
```
dependency_audit({ path: "<project_root>", run_scanners: true })
```

### Step 4.2: Pattern Analysis
Look for project-wide patterns:
- Are parameterized queries used consistently?
- Is input validation applied at the framework level or ad-hoc?
- Are there centralized sanitization functions?

```
query_cpg({
  path: "<project_root>",
  query: { type: "search", query: "sanitize" }
})
query_cpg({
  path: "<project_root>",
  query: { type: "search", query: "validate" }
})
```

## Phase 5: Tenth Man Review & Reporting

### Step 5.1: Challenge Findings
```
tenth_man_review({
  topic: "Code Audit Findings Review",
  findings: "<summary of all findings>"
})
```
The Tenth Man protocol challenges all findings to identify:
- False positives
- Missing findings
- Severity over/under-estimation

### Step 5.2: Generate Report
```
audit_report({
  path: "<project_root>",
  format: "markdown",
  include_recommendations: true
})
```

## Key Principles

1. **Graph first, then code**: Use `query_cpg` to understand structure before reading source files
2. **Automated scan, then manual verification**: Let `cpg_security_scan` find candidates, then verify each
3. **Follow the data**: Use `cpg_taint_analysis` to trace data flow, not just grep for patterns
4. **Check the sanitizers**: A sanitized finding is not necessarily safe — verify the sanitizer is effective
5. **Cross-file awareness**: Use `callers_of` and `callees_of` to understand the full attack surface
6. **Document everything**: Use `audit_finding_upsert` for confirmed vulns, `audit_coverage` for progress
