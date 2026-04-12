---
name: payment-flow
description: Analyze payment and refund workflows for skipped prerequisites, replay issues, amount inconsistencies, and callback abuse.
when_to_use: Use when requests and process graphs suggest order creation, payment, callback, refund, settlement, or balance deduction behavior.
---
Focus on payment-style business flows.

Key invariants:
- Payment should normally require a valid unpaid or pending business state.
- Refund should normally require a successful paid or settled state.
- Amount, currency, and target order/resource identifiers should remain consistent across steps.
- Callback or notification requests should not be trusted without server-side validation.
- Repeating the same payment or refund action should not succeed multiple times without explicit idempotency semantics.

Recommended analysis:
- Identify objects like orderId, paymentId, transactionId, amount, currency, couponId.
- Compare sequence ordering across create, pay, callback, refund, cancel, settle.
- Look for cases where the same actor can trigger pay/refund repeatedly with similar payloads.
- Flag flows where success responses occur despite missing prerequisite states.

Recommended verification strategies:
- repeat_action
- swap_resource_reference
- manual_review for callback forgery, amount tampering, or multi-step balance inconsistencies
