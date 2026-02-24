import type { Role } from '@/types/role'

export const AUDIT_LIFECYCLE_TRANSITION_CAPABILITY = 'audit.lifecycle.transition'

export const isAuditRole = (role: Role | null | undefined): boolean => {
  if (!role) return false
  if (role.is_system) return true

  return Array.isArray(role.capabilities)
    ? role.capabilities.includes(AUDIT_LIFECYCLE_TRANSITION_CAPABILITY)
    : false
}

export const canOperateAuditLifecycle = (
  role: Role | null | undefined,
  overrideEnabled = false,
): boolean => {
  if (overrideEnabled) return true
  return isAuditRole(role)
}
