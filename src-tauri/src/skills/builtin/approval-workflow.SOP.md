---
name: approval-workflow
description: Analyze approval-style flows for role separation failures, missing state checks, duplicate approvals, and workflow bypass.
when_to_use: Use when requests suggest submit, approve, reject, review, assign, sign-off, publish, or escalation actions.
---
Focus on approval and review workflows.

Key invariants:
- Approval and rejection actions should require an authorized role.
- Approval should usually depend on a prior submit, draft, or pending-review state.
- Repeated approval or reject actions should not succeed indefinitely.
- Users should not be able to approve their own protected actions unless explicitly allowed.

Recommended analysis:
- Identify role, approver, reviewer, submitter, owner, and object identifiers.
- Compare distinct principals against the same object and same action family.
- Look for approve/reject success responses from low-privilege or unrelated principals.
- Flag success responses when processGraph suggests a missing prerequisite transition.

Recommended verification strategies:
- replay_as_is
- repeat_action
- swap_resource_reference
- manual_review for multi-role or multi-step sequence abuse
