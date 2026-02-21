# Common Security Audit Checklist

## Triage First

- Prioritize internet-exposed code paths.
- Prioritize dangerous sinks: SQL, command execution, template render, file write/read, HTTP internal calls.
- Map trusted vs untrusted boundaries.

## Severity Heuristic

- Critical: Remote compromise, auth bypass, or sensitive data takeover with low complexity.
- High: Significant privilege/data impact with practical exploitation.
- Medium: Real weakness with constraints or partial impact.
- Low: Hard-to-exploit or defense-in-depth gaps.

## Evidence Standards

- Show source (input) to sink (dangerous operation) flow.
- Show missing or broken control.
- Include at least one realistic abuse scenario.
- Mark if issue is framework-default dependent.

## Mandatory Categories

- Authentication/session flaws
- Authorization/tenant isolation flaws
- Injection (SQL/NoSQL/LDAP/command/template)
- Deserialization and unsafe parsing
- SSRF and outbound request abuse
- File upload/path traversal
- Cryptography misuse
- Secret leakage and credential handling
- Concurrency/race logic
- Dependency risk and vulnerable packages

## False Positive Filters

- Do not flag safe ORM prepared statements as SQL injection.
- Do not flag escaped output when context-aware escaping is correctly used.
- Do not flag intentionally public endpoints as IDOR without sensitive action exposure.

## Remediation Style

- Give minimal patch first.
- Provide safer alternative pattern if architecture allows.
- Mention migration impact briefly.
