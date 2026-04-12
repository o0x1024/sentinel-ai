---
name: single-use-consumption
description: Analyze single-use or idempotent actions for duplicate execution, race conditions, and repeatable consumption bugs.
when_to_use: Use when requests suggest redeem, consume, claim, bind, activate, submit, or other one-time operations.
---
Focus on single-use, idempotent, and race-sensitive actions.

Key invariants:
- Single-use resources should not be consumed multiple times.
- Duplicate or rapid repeated requests should not all succeed.
- Concurrent execution should not create inconsistent or duplicate business effects.

Recommended analysis:
- Identify couponId, token, redeem code, orderId, inviteId, activationId, and similar one-time identifiers.
- Check whether the same actor repeats an action with very similar payloads.
- Look for success responses across repeat actions in short time windows.

Recommended verification strategies:
- repeat_action
- swap_resource_reference
- manual_review for explicit concurrency or multi-step replay scenarios
