import { describe, expect, it } from 'vitest'
import { canOperateAuditLifecycle, isAuditRole } from '@/utils/auditLifecycleAccess'
import type { Role } from '@/types/role'

const mkRole = (overrides: Partial<Role>): Role => ({
  id: 'r1',
  title: 'General Assistant',
  description: 'General purpose role',
  prompt: 'Help users with coding',
  capabilities: [],
  is_system: false,
  created_at: new Date(),
  updated_at: new Date(),
  ...overrides,
})

describe('audit lifecycle access', () => {
  it('grants permission to system role', () => {
    const role = mkRole({ is_system: true })
    expect(isAuditRole(role)).toBe(true)
    expect(canOperateAuditLifecycle(role, false)).toBe(true)
  })

  it('grants permission when explicit lifecycle capability exists', () => {
    const role = mkRole({
      capabilities: ['audit.lifecycle.transition'],
    })
    expect(isAuditRole(role)).toBe(true)
    expect(canOperateAuditLifecycle(role, false)).toBe(true)
  })

  it('denies permission when capability is absent', () => {
    const role = mkRole({
      capabilities: ['agent.chat'],
    })
    expect(isAuditRole(role)).toBe(false)
    expect(canOperateAuditLifecycle(role, false)).toBe(false)
  })

  it('allows explicit override for emergency/manual ops', () => {
    const role = mkRole({
      title: 'Product Planner',
      description: 'Business planning and roadmap',
    })
    expect(canOperateAuditLifecycle(role, true)).toBe(true)
    expect(canOperateAuditLifecycle(null, true)).toBe(true)
  })
})
