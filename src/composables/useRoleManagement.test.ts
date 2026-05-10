import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.hoisted(() => vi.fn())

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}))

const role = {
  id: 'role-1',
  title: 'Role 1',
  description: 'Primary role',
  prompt: 'You are useful.',
  capabilities: ['shell'],
  is_system: false,
  created_at: new Date('2026-01-01T00:00:00.000Z'),
  updated_at: new Date('2026-01-01T00:00:00.000Z'),
}

describe('useRoleManagement', () => {
  beforeEach(() => {
    vi.resetModules()
    invokeMock.mockReset()
    window.localStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('restores the backend-selected role even when localStorage quota is exhausted', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_ai_roles') return Promise.resolve([role])
      if (command === 'get_current_ai_role') return Promise.resolve(role)
      return Promise.reject(new Error(`Unexpected command: ${command}`))
    })
    vi.spyOn(window.localStorage.__proto__, 'setItem').mockImplementation(() => {
      throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
    })
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    const { useRoleManagement } = await import('./useRoleManagement')
    const manager = useRoleManagement()

    await expect(manager.loadRoles()).resolves.toBeUndefined()

    expect(manager.selectedRole.value?.id).toBe('role-1')
    expect(consoleErrorSpy).not.toHaveBeenCalledWith(
      'Failed to restore selected role:',
      expect.anything(),
    )
  })

  it('keeps role selection successful when optional localStorage persistence fails', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'set_current_ai_role') return Promise.resolve(null)
      return Promise.reject(new Error(`Unexpected command: ${command}`))
    })
    vi.spyOn(window.localStorage.__proto__, 'setItem').mockImplementation(() => {
      throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
    })

    const { useRoleManagement } = await import('./useRoleManagement')
    const manager = useRoleManagement()

    await expect(manager.selectRole(role)).resolves.toBeUndefined()

    expect(manager.selectedRole.value?.id).toBe('role-1')
    expect(invokeMock).toHaveBeenCalledWith('set_current_ai_role', { roleId: 'role-1' })
  })
})
