# PHP Vulnerability Patterns

Use this file as a quick triage sheet while auditing.

## Sources (untrusted input)

- Superglobals: `$_GET`, `$_POST`, `$_REQUEST`, `$_COOKIE`, `$_FILES`, `$_SERVER`
- Raw body parsers, JSON payloads, headers, webhook payloads
- Queue messages, cron arguments, imported CSV/Excel content

## High-Risk Sinks and Typical Fixes

1. SQL injection
- Risk patterns: dynamic SQL with string concatenation/interpolation.
- Preferred fix: parameterized queries (`PDO` prepared statements or framework query builder bindings).

2. Command injection
- Risk patterns: `system`, `exec`, `shell_exec`, `passthru`, `proc_open`, backticks with user-influenced input.
- Preferred fix: avoid shell; use native APIs. If unavoidable, strict allowlists and argument escaping.

3. XSS
- Risk patterns: raw echo in HTML/attribute/JS contexts, unescaped template output.
- Preferred fix: context-aware encoding (`htmlspecialchars` for HTML context, proper JS/URL encoding where needed).

4. Unsafe deserialization / object injection
- Risk patterns: `unserialize` on untrusted content; dangerous magic methods (`__wakeup`, `__destruct`).
- Preferred fix: use JSON for untrusted data; avoid unserialize or use strict class allowlist and redesign flow.

5. File inclusion and path traversal
- Risk patterns: user-controlled include path, file path join without canonicalization.
- Preferred fix: fixed allowlist map, canonical path checks, deny traversal tokens, isolate storage roots.

6. File upload abuse
- Risk patterns: extension-only checks, web-reachable upload dirs, trusting client MIME.
- Preferred fix: server-side MIME/content validation, random file names, non-executable storage, strict size limits.

7. CSRF / authz gaps
- Risk patterns: state-changing endpoints without CSRF token, missing per-object authorization checks.
- Preferred fix: CSRF middleware/token checks; policy-based access control for every sensitive object action.

8. SSRF
- Risk patterns: arbitrary URL fetchers with no destination control.
- Preferred fix: protocol/domain/IP allowlists, block internal address ranges, disable redirects or re-validate target.

## False-Positive Filters

- Verify whether framework auto-escaping is active in the exact rendering context.
- Verify whether ORM/query builder actually binds parameters (not raw fragments).
- Verify whether middleware/policy is enforced on the exact route and action path.

## Reporting Standard

For each finding, record:

- Entry point
- Data flow to sink
- Missing control
- Exploitability precondition
- Minimal fix and regression test idea
