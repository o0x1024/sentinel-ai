---
name: resource-ownership
description: Analyze resource access patterns for ownership mismatches, cross-tenant access, and object-level authorization failures.
when_to_use: Use when requests include resource identifiers, ownership hints, or cross-principal access to the same objects.
---
Focus on ownership and authorization boundaries.

Key invariants:
- Object identifiers should be scoped to the authenticated principal, tenant, or organization.
- Read and write operations on the same object should not succeed across unrelated principals.
- Sensitive actions on resources should enforce both object and function authorization.

Recommended analysis:
- Track userId, tenantId, orgId, projectId, orderId, accountId, roleId, and similar identifiers.
- Compare access outcomes across principals and auth contexts.
- Prioritize mismatches where the same resource returns success across clearly distinct identities.

Recommended verification strategies:
- swap_resource_reference
- replay_as_is
- manual_review for multi-tenant or chained privilege escalation cases
