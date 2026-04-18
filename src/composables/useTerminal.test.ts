import { describe, expect, it } from 'vitest'

import { buildTerminalSessionFingerprint, useTerminal } from './useTerminal'

describe('useTerminal', () => {
  it('syncs active session and fingerprint explicitly', () => {
    const terminal = useTerminal()
    terminal.resetTerminal()

    terminal.syncActiveSession('session-1', 'docker|image|bash')

    expect(terminal.currentSessionId.value).toBe('session-1')
    expect(terminal.currentSessionFingerprint.value).toBe('docker|image|bash')
    expect(terminal.hasHistory.value).toBe(true)
  })

  it('clears fingerprint when active session is reset', () => {
    const terminal = useTerminal()
    terminal.resetTerminal()

    terminal.syncActiveSession('session-1', 'docker|image|bash')
    terminal.syncActiveSession(null, null)

    expect(terminal.currentSessionId.value).toBeNull()
    expect(terminal.currentSessionFingerprint.value).toBeNull()
  })

  it('builds terminal session fingerprints deterministically', () => {
    expect(buildTerminalSessionFingerprint('docker', 'Sentinel-Sandbox:Latest', 'Bash')).toBe(
      'docker|sentinel-sandbox:latest|bash',
    )
  })
})
