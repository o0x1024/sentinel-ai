import { beforeEach, describe, expect, it, vi } from 'vitest'

describe('useAgentSessionManager', () => {
  beforeEach(() => {
    vi.resetModules()
    window.localStorage.clear()
  })

  it('replaces the active session id and persists the new id', async () => {
    const { useAgentSessionManager } = await import('./useAgentSessionManager')
    const manager = useAgentSessionManager()

    manager.addSession('conv-old', 'Old conversation')
    manager.replaceSession('conv-old', 'conv-new', 'New conversation')

    expect(manager.sessions.value).toEqual([
      {
        id: 'conv-new',
        title: 'New conversation',
        isActive: false,
      },
    ])
    expect(manager.activeSessionId.value).toBe('conv-new')
    expect(JSON.parse(window.localStorage.getItem('ai:session-manager') || '{}')).toMatchObject({
      activeSessionId: 'conv-new',
      sessions: [
        {
          id: 'conv-new',
          title: 'New conversation',
        },
      ],
    })
  })

  it('deduplicates when replacing into an existing session id', async () => {
    const { useAgentSessionManager } = await import('./useAgentSessionManager')
    const manager = useAgentSessionManager()

    manager.addSession('conv-a', 'Conversation A')
    manager.addSession('conv-b', 'Conversation B')
    manager.replaceSession('conv-a', 'conv-b', 'Conversation B merged')

    expect(manager.sessions.value).toEqual([
      {
        id: 'conv-b',
        title: 'Conversation B merged',
        isActive: false,
      },
    ])
    expect(manager.activeSessionId.value).toBe('conv-b')
  })
})
