---
name: ctf-web-solver
description: CTF Web challenge analysis and exploitation workflow for reconnaissance, vulnerability hypothesis, payload crafting, response-driven branching, and flag extraction. Use when users ask to solve Web CTF tasks, debug failed payloads, audit challenge source code, bypass filters/WAF rules, chain multiple web vulnerabilities, or improve CTF Web solving efficiency and accuracy.
---

# CTF Web Solver

Use this skill to solve CTF Web challenges with a verification-first process.

## Workflow

1. Clarify challenge scope and win condition.
2. Build attack surface map from traffic and source.
3. Generate prioritized vulnerability hypotheses.
4. Run minimal proofs before deep exploitation.
5. Branch by observed responses and iterate.
6. Extract flag and summarize reproducible path.

## Scope And Inputs

- Gather challenge description, endpoint list, known credentials, and target flag format.
- Gather exact requests and responses for each attempted payload.
- Gather source snippets, framework info, middleware, and filter logic when available.
- Record constraints: blocked chars, length limits, encoding transforms, and WAF behavior.

## Prioritized Vulnerability Ladder

1. Sensitive file exposure and debug artifacts
2. Authentication/session flaws and logic bypass
3. Injection class issues (SQL/NoSQL/Command/Template)
4. Deserialization and parser abuse
5. SSRF, file upload, and path traversal
6. Access control and multi-step business logic flaws

If one lane fails, keep evidence and move to the next lane quickly.

## Output Format For Each Attempt

- Hypothesis: what weakness is being tested
- Payload/request: exact request to send
- Expected signal: status/body/timing/header change
- Actual signal: observed response
- Decision: continue, pivot, or discard

Never claim success without a response-based proof.

## Branching Rules

- If response reflects sanitized input, try encoding, delimiter smuggling, and context escapes.
- If response is generic error, reduce payload to a minimal syntax probe and recover parser behavior.
- If endpoint appears dead, compare method, host header, content type, and auth state.
- If blind channel only, shift to timing, out-of-band, or side-effect verification.

## Payload Construction Discipline

- Start with one-variable probes before full exploit chains.
- Change one factor at a time (encoding, wrapper, function, transport).
- Keep a payload matrix with request ID and result.
- Reuse only payloads that showed partial positive signals.

## Code-Aware Review Rules

- Trace input source -> transformation -> sink.
- Identify trust boundary crossings and type conversions.
- Validate preconditions for exploitability, not just sink presence.
- Prefer exploit paths that require the fewest assumptions.

## Load References On Demand

- For reconnaissance and endpoint mapping, read `references/recon.md`.
- For exploit patterns and fallback branches, read `references/exploitation.md`.

## Final Delivery Requirements

- Provide shortest reproducible solve path.
- Include exact requests that worked.
- Explain why earlier attempts failed.
- Add one hardened fix recommendation per confirmed flaw.
