---
name: php-code-audit
description: Audit PHP codebases for exploitable security vulnerabilities and provide actionable fixes. Use when reviewing PHP applications, pull requests, legacy modules, or incident-related code paths for issues such as SQL injection, command injection, XSS, unsafe deserialization, file inclusion, path traversal, auth/session flaws, CSRF, SSRF, and insecure file upload handling.
---

# PHP Code Audit

## Overview

Perform security-focused code review for PHP projects, prioritize exploitable findings, and produce fix-ready remediation guidance.

## Audit Workflow

1. Confirm runtime and architecture context.
- Identify PHP version, framework (Laravel/Symfony/ThinkPHP/WordPress/custom), web server, and key middleware.
- Locate request entry points, auth boundaries, upload endpoints, background jobs, and admin-only features.

2. Build trust-boundary and data-flow map.
- Trace untrusted inputs from `$_GET`, `$_POST`, `$_REQUEST`, `$_COOKIE`, `$_FILES`, headers, CLI args, and message queues.
- Track transformation layers (validation, casting, encoding, ORM/query builders, templates) before dangerous sinks.

3. Hunt high-risk vulnerability classes first.
- Check injection surfaces: SQL/NoSQL, OS command, template injection, LDAP, XPath.
- Check code execution primitives: `eval`, `assert`, dynamic include/require, `preg_replace` `/e` legacy usage, unsafe callback dispatch.
- Check file and path issues: upload validation bypass, MIME spoofing, extension bypass, path traversal, symlink abuse.
- Check auth/session issues: broken access control, IDOR, weak session handling, insecure token lifecycle.
- Check web abuse: reflected/stored/DOM XSS, CSRF gaps, SSRF with internal target reachability.
- Check crypto and secret handling: hardcoded keys, weak hashing, insecure randomness, unsafe encryption mode/config.

4. Validate exploitability and blast radius.
- Confirm whether attacker-controlled data reaches a sink without robust validation or context-safe encoding.
- Distinguish true positives from framework-provided protections.
- Estimate impact scope: privilege required, affected roles, data sensitivity, remote exploitability.

5. Provide patch-ready output.
- For each finding, include: severity, CWE, evidence path, vulnerable flow, exploit preconditions, and concrete remediation.
- Prefer minimal, local, testable fixes; add safer alternatives when architectural changes are needed.
- Include regression-test suggestions per finding.

## Output Contract

Produce results in this structure:

1. `Summary`: total findings by severity + top systemic risks.
2. `Findings`: one item per issue with `ID`, `Severity`, `CWE`, `Location`, `Why exploitable`, `Fix`, `Suggested test`.
3. `Missed-by-automation risks`: logic flaws or access-control assumptions requiring manual validation.
4. `Next passes`: most valuable follow-up review targets.

## Review Heuristics

- Treat direct string concatenation into SQL, shell, include paths, and HTML/JS contexts as suspicious by default.
- Prefer framework-native defenses (prepared statements, policy gates, escaping helpers, CSRF middleware) over custom filters.
- Mark uncertainty explicitly and request the minimum extra context needed to confirm exploitability.

## References

- Load `references/vuln-patterns.md` for sink/source quick checks and remediation patterns.
