# Orchestration Event Migration Guide

This document describes the migration from the legacy `UnifiedStreamMessage` pipeline to the structured `OrchestrationEvent` system.

## Goals
- Provide richer, typed execution lifecycle visibility (plan, steps, tools, replans, metrics, summary)
- Support dynamic plan versioning + replan transparency
- Maintain dual-write for safe rollback until feature parity & stability are verified
- Reduce frontend parsing complexity and ambiguity present in ad‑hoc streamed text blocks

## Legacy → New Mapping
| Legacy Concept | UnifiedStreamMessage (old) | OrchestrationEvent (new) | Notes |
| -------------- | -------------------------- | ------------------------ | ----- |
| Plan emission | Initial markdown plan blob | `PlanSnapshot` | Includes `plan_version` + optional `reason` (e.g. replan) |
| Step start | Implicit / inferred from messages | `StepUpdate { status: Running }` | Explicit ID & timing |
| Step complete | Text summary appended | `StepUpdate { status: Completed }` | Carries optional result/metrics fields (extensible) |
| Step failure | Mixed error text | `StepUpdate { status: Failed, error }` | Enables structured failure handling |
| Tool started | Sometimes inline code block | `ToolUpdate { status: Started }` | Deterministic correlation via `tool_call_id` |
| Tool output / finish | Inline streaming chunks | `ToolUpdate { status: Completed, output }` | Future: partial streaming channel if needed |
| Tool failure | Error paragraph | `ToolUpdate { status: Failed, error }` | Distinguishes failure vs timeout |
| Metrics pulse | None / implicit | `Metrics` | Currently: active_steps, failures, retries |
| Replan request | Not supported | `ReplanProposed` | Triggered by heuristic (failed steps) |
| Replan accepted | Manual / none | `ReplanApplied` + new `PlanSnapshot` | Increments `plan_version` |
| Replan rejected | N/A | `ReplanRejected { reason }` | Explicit negative path |
| Final summary | Last assistant message | `FinalSummary` | Marked terminal; includes success flag |

## Event Meta Fields
All events include:
- `session_id`
- `sequence` (monotonic per session)
- `plan_version` (1+; 0 = pre-plan edge cases)
- `engine_kind`
- `is_terminal` (true only for `FinalSummary` currently)
- `timestamp` (ISO-8601)

## Plan Version Semantics
| Action | Function | Result |
| ------ | -------- | ------ |
| First plan emission | `init_plan_version` | Returns existing or initializes to `1` |
| Emit events referencing active plan | `current_plan_version` | Read without mutation |
| Successful replan acceptance | `next_plan_version` | Atomically increments (N→N+1) |

## Replan Flow Outline
1. Failure heuristic triggers (e.g. at least one failed step)
2. Emit `ReplanProposed`
3. Attempt replanner logic
4. If new plan produced → `ReplanApplied` then `PlanSnapshot` (new version)
5. Else → `ReplanRejected { reason }`

## Dual-Write Strategy
Phase 1 (Current): Emit both legacy `ai_stream_message` & new `orchestration_event` (synthetic `message_id` included for UI compatibility).
Phase 2: Frontend shifts primary rendering to orchestration store; legacy used only for chat transcript fallback.
Phase 3: Deprecate most legacy emissions except minimal transcript summarization.
Phase 4: Remove legacy path after stability window + telemetry confirmation.

## Rollback Procedure
1. Keep legacy emission code paths untouched until Phase 3 exit criteria met.
2. If regression detected: hide orchestration panel (feature flag) & rely on legacy stream; no backend change needed.
3. For severe backend issues: temporarily disable event emission wrapper (no crash risk—events are best-effort).

## Testing Checklist
- [x] PlanSnapshot appears first with plan_version=1
- [x] StepUpdate lifecycle (Running → Completed/Failed)
- [x] ToolUpdate coverage (Started → Completed/Failed/Timeout)
- [x] Metrics pulses interleave without breaking ordering
- [x] Replan cycle (Proposed → Applied + new Snapshot) increments version
- [x] ReplanRejected path tested (no new plan available)
- [x] FinalSummary always terminal & last sequence

## Frontend Consumption
Pinia store (`orchestrationStore`) indexes by `session_id` & groups by `plan_version`. The new `ExecutionOrchestrationPanel.vue` renders:
- Current plan steps & statuses
- Active / recent tool calls
- Metrics header summary
- Replan banner when a replan cycle is in-flight or recently resolved
- Timeline (sequence-ordered condensation—future: virtualized list or time axis)

## Extensibility Roadmap
Short-term:
- Richer plan diffs (added/removed/modified steps)
- Tool streaming partials (chunked output events)
- Aggregated metrics spans (rate & latency histograms)
- Artifact events (files, structured outputs) distinct from FinalSummary
Medium:
- Step retry events & policy metadata
- Correlated trace IDs for distributed tool chains
- Throttled metrics (heartbeat cadence vs per-step)
Long-term:
- Multi-engine orchestration spans
- Deterministic replay (event log → state reconstruction)

## Deprecation Criteria for Legacy Stream
Proceed to Phase 3 removal when:
- 95%+ of UI features use orchestration store
- No open P0/P1 bugs on event ordering
- Plan diffs MVP shipped
- Metrics aggregation baseline accepted

## Developer Notes
- All emission helpers MUST acquire `plan_version` at call time (avoid stale captured values)
- Do not treat missing AppHandle as fatal—log at debug & continue
- Keep sequence strictly monotonic; never reuse or decrement
- Prefer additive enum variants to breaking changes; version via `plan_version` semantics, not enum re-tagging

## FAQ
Q: Why not reuse legacy unified message with embedded JSON?\nA: Ambiguous parsing, coupling presentation with transport, and inability to version substreams cleanly.

Q: How to add a new event type?\nA: Extend enum + payload struct in `engines/types.rs`, update emitter, add store handler & UI renderer component.

Q: How to correlate legacy transcript lines to events?\nA: Use synthetic `message_id` (sequence-based) during transition; final design may hide this once legacy path removed.

---
Maintainers: Update this document as new phases complete or format/field changes are introduced.

## (Update) Extended Event Model Additions

The following extensions were introduced to improve streaming granularity and artifact lifecycle clarity:

### PlanDiffAction Enum
Replaces free-form `change` string with a strict enum for safer client logic:
- `Add` | `Remove` | `Modify` | `Reorder`
Additional contextual fields:
- `new_index` (for Add / Reorder)
- `old_index` (for Reorder)
- `before` / `after` textual summaries for Modify/Reorder when available.

### ToolOutputChunk (kind = toolOutputChunk)
Purpose: Support incremental tool output streaming without overloading a single ToolUpdate.
Fields:
- `tool_call_id`, `step_id` (optional)
- `sequence_in_tool` (monotonic within that tool invocation)
- `delta` (raw text fragment)
- `is_last` (marks completion; final ToolUpdate Completed still recommended for terminal state)

Client Handling Guideline:
1. Accumulate `delta` into a tool message buffer keyed by `tool_call_id`.
2. Mark buffer complete when `is_last=true` OR a subsequent `ToolUpdate Completed` arrives.

### ArtifactPublished (kind = artifactPublished)
Distinguished from `artifactRef`:
- `artifactRef` is a lightweight reference (e.g. early knowledge of an upcoming artifact or cross-link)
- `artifactPublished` signals artifact is fully materialized (size/digest stable)

Fields:
- `artifact_id`, `kind`, `label`
- Optional: `mime`, `size`, `digest`, `preview`, `step_id`

### Migration Impact
Existing consumers that only switch on previously known `kind` values should default-safe ignore new kinds. Frontend store updated to:
- Merge `toolOutputChunk` into existing tool message stream
- Render `artifactPublished` & `artifactRef` both as artifact timeline entries (different IDs)

### Backward Compatibility
- No existing variants were renamed.
- `PlanDiff.action` added; if older backend produced `change` only, a converter layer (not implemented here) would be required. Current code paths exclusively emit new structure.

### Future Considerations
- Potential `StepOutputChunk` analogous to tool streaming.
- Artifact binary transfer handshake (separate channel) with progressive integrity events.

