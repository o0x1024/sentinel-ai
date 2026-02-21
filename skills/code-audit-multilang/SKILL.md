---
name: code-audit-multilang
description: Multi-language code security audit and secure coding review for PHP, Java, Go (Golang), Node.js/JavaScript/TypeScript, and Python. Use when users ask to audit code, perform security review, find vulnerabilities, assess risky logic, check authentication/authorization/session issues, validate input/output handling, review dependency and supply-chain risk, or produce remediation guidance and proof-of-concept test cases.
---

# Code Audit Multilang

Use this skill to run a practical, evidence-based security audit across mixed codebases.

## Workflow

1. Scope the audit target.
2. Detect framework and language entry points.
3. Run static review with vulnerability categories first.
4. Validate exploitability and reachable paths.
5. Produce prioritized findings with concrete fixes.

## Scope The Audit Target

- Confirm files/modules/in-scope services.
- Confirm threat model quickly: external attacker, internal user, CI/CD, or supply chain.
- Confirm expected output format:
  - Full report
  - High-risk-only findings
  - Patch suggestions
  - Verification checklist

## Analyze In This Order

1. Authentication, session, token lifecycle
2. Authorization and privilege boundaries
3. Input validation and deserialization
4. Data access and injection vectors
5. File, command, template, and SSRF surfaces
6. Cryptography and secret management
7. Business-logic abuse and race conditions
8. Dependency and third-party integration risk
9. Logging, monitoring, and sensitive-data exposure

## Load References On Demand

- For shared methodology and scoring, read `references/common.md`.
- For language checks, load only the needed files:
  - `references/php.md`
  - `references/java.md`
  - `references/golang.md`
  - `references/nodejs.md`
  - `references/python.md`
- For output structure, read `references/report-template.md`.

## Finding Quality Bar

- Include exact file path and line.
- Explain attack preconditions.
- Show impact in plain language.
- Provide minimal safe fix and longer-term hardening option.
- State confidence and how to reproduce/verify.
- Do not report speculative issues without evidence.

## Output Requirements

- Group by severity: Critical, High, Medium, Low.
- For each finding, include:
  - Title
  - Affected location
  - Why vulnerable
  - Exploit path
  - Remediation
  - Confidence
- End with:
  - Quick wins (can fix today)
  - Structural fixes (architecture/process)
  - Suggested tests to prevent regression
