# AI Assistant Turn Execution Plan

## Problem

The current AI Assistant execution path lets one identifier act as both the conversation container and the concrete execution run. That makes three different facts share the same key:

- Conversation history: where messages belong.
- Turn execution: the currently running user request.
- Task ledger: whether the current work is complete.

When a previous task is complete and the user sends a new message such as "continue", old completion state can still influence the new turn. When a previous task is interrupted, the same ambiguity can make a fresh message accidentally replace or disturb unfinished work.

The first-principles boundary is:

- `conversation_id` is the long-lived context container.
- `execution_id` / `turn_id` is one concrete user submission and model run.
- `execution_tasks` is completion truth for that execution, not for the whole conversation.

## Short-Term Plan

### Phase 0: Execution Plan and Current Evidence

Status: implemented for the current short-term slice on 2026-05-08.

Verification:

- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm test -- --run agentConversationExecutionSupport.test.ts useAgentEvents.test.ts`

### Phase 1: Separate Execution Identity

Goal: every new user submission gets a fresh `execution_id`, while messages still persist under the existing `conversation_id`.

Implementation:

- Add `execution_id` to the frontend `agent_execute` config.
- Generate a fresh execution ID for each normal submit.
- Keep `conversation_id` for conversation creation, history loading, RAG context, and message persistence.
- Use `execution_id` for cancellation, stream chunks, tool events, task ledger, harness completion checks, and final execution settlement.
- Include `conversation_id` on runtime events so the UI can route turn-scoped events into the active conversation.

Acceptance checks:

- A completed previous turn cannot settle a new turn.
- A new user message in the same conversation gets a different `execution_id`.
- Tool task rows are keyed by the new execution ID.
- Assistant/user messages still appear in the original conversation.

### Phase 2: Frontend Execution Guard

Goal: prevent concurrent turn confusion in the UI.

Implementation:

- Track the current active execution ID from runtime events.
- Stop/cancel by active execution ID, not conversation ID.
- Accept events when their `conversation_id` matches the current conversation, but settle only the active `execution_id`.
- Reject stale events whose execution ID is neither the active turn nor the current conversation.

Acceptance checks:

- Stale cancelled/finished events from an older turn do not clear the current turn.
- Running-state UI follows `agent:execution_finished`, not `agent:assistant_message_saved`.
- The stop button cancels the active execution.

### Phase 3: Runtime Continue Resolver

Goal: make "continue" deterministic before it reaches the model.

Status: implemented for the first runtime-backed slice on 2026-05-08.

Implementation:

- Inspect the most recent turn for the current conversation.
- If the latest turn is `in_progress`, `interrupted`, or `incomplete` and has unfinished `execution_tasks`, treat "continue" as resume.
- If the latest turn is `succeeded` or `completed`, treat "continue" as a new turn.
- If state is contradictory, surface an explicit UI decision instead of guessing.
- Use `agent_harness_runs` as the current durable turn ledger, with run ID aligned to `execution_id`.
- Block a continue-only input while the frontend still has an active execution instead of taking over and stopping it.

Acceptance checks:

- "continue" after completed work starts a normal new turn.
- "continue" after network interruption resumes unfinished ledger work.
- The model is not asked to infer resume state from text alone.

### Phase 4: Split Oversized Execution Files

Goal: reduce risk in the event/runtime layer by splitting files that exceed 2000 lines into function-owned modules.

Status: partially implemented. Team V4 flow helpers were split out of `useAgentConversationFlow.ts`, and event support helpers were split out of `useAgentEvents.ts`; both files are now below 2000 lines.

Targets:

- Continue splitting `useAgentEvents.ts` by stream events, tool events, subagent events, and context-compression events when those areas are next changed.
- Continue splitting `useAgentConversationFlow.ts` only when new ownership boundaries are touched.
- Split `ai.rs` and `run_with_tools.rs` by message streaming, cancellation, completion emission, and persistence support.

Acceptance checks:

- No behavior change during the split.
- Existing tests continue to pass.
- New code paths import function-level helpers instead of growing the large files further.

## Long-Term Plan

### Durable Turn Ledger

Create an `agent_execution_turns` table:

- `turn_id`
- `conversation_id`
- `user_message_id`
- `task_ledger_id`
- `parent_turn_id`
- `status`
- `interruption_reason`
- `created_at`
- `completed_at`

This gives the runtime a durable state machine instead of reconstructing truth from UI state or final text.

### Checkpointed Resume

Persist checkpoints for:

- `turn_started`
- `user_message_saved`
- `tool_started`
- `tool_completed`
- `task_updated`
- `assistant_message_saved`
- `completion_verified`
- `execution_finished`

Resume must restore from checkpoint state, not from the last visible message alone.

Status: first durable slice implemented on 2026-05-08.

- Added `agent_execution_turns` as the durable turn ledger.
- Runtime start records `turn_id`, `conversation_id`, `user_message_id`, task, status, and harness mode.
- Harness checkpoints mirror the latest checkpoint type and payload into the turn ledger.
- Continue resolution now checks the turn ledger before falling back to harness runs.

### Completion Verifier

The backend must verify completion before success emission:

- Planned/team modes must have a task ledger.
- `pending` or `in_progress` tasks block success.
- `failed` tasks produce failed execution.
- Repeated continuations with unchanged ledger state become `stalled_without_ledger_progress`.

Status: first enforcement slice implemented on 2026-05-08.

- Final harness assessment writes failed/incomplete/stalled state into `agent_execution_turns`.
- Max-continuation exhaustion now settles as `stalled_without_ledger_progress`.
- `agent:execution_finished` persistence mirrors the final outcome into the turn ledger as a secondary settlement path.

### Queue and Interrupt UX

Running-turn input should be explicit:

- Default: queue as next turn.
- Explicit action: interrupt current turn and send now.
- Explicit action: resume unfinished turn.

The UI should show queued messages and active execution ID in diagnostics.

Status: first UX slice implemented on 2026-05-08.

- Enter/send while an execution is active queues one next-turn message instead of stopping the active run.
- Continue-only input while active execution is running is blocked with an explicit message.
- A dedicated interrupt-send action is available for the explicit "stop current and send now" path.

## Known Risks

- The first phase touches both frontend and Rust runtime paths; partial rollout can create event routing gaps.
- Existing persisted records do not have true turn IDs. Migrate old records into legacy turn rows once the new table exists.
- Tool execution historically used `conversation_id` as its execution key, so tool message persistence must preserve conversation storage while tool state moves to execution identity.

## Implementation Order

1. Add explicit `execution_id` to new executions.
2. Route event filtering by active execution ID plus conversation ownership.
3. Move cancellation and task ledger reads to execution ID.
4. Add durable turn records.
5. Add continue resolver.
6. Add checkpointed resume.
7. Add stalled-ledger verifier.
