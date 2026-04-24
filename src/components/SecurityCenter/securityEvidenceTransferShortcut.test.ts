import { describe, expect, it } from 'vitest'
import { resolveSecurityEvidenceTransferShortcut } from './securityEvidenceTransferShortcut'

describe('resolveSecurityEvidenceTransferShortcut', () => {
  it('maps Cmd/Ctrl+I to intruder', () => {
    const target = resolveSecurityEvidenceTransferShortcut({
      key: 'i',
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      defaultPrevented: false,
      repeat: false,
      isComposing: false,
      target: document.body,
    })

    expect(target).toBe('intruder')
  })

  it('maps Cmd/Ctrl+R to repeater', () => {
    const target = resolveSecurityEvidenceTransferShortcut({
      key: 'r',
      metaKey: false,
      ctrlKey: true,
      altKey: false,
      shiftKey: false,
      defaultPrevented: false,
      repeat: false,
      isComposing: false,
      target: document.body,
    })

    expect(target).toBe('repeater')
  })

  it('ignores shortcuts while editing text', () => {
    const input = document.createElement('input')

    const target = resolveSecurityEvidenceTransferShortcut({
      key: 'i',
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      defaultPrevented: false,
      repeat: false,
      isComposing: false,
      target: input,
    })

    expect(target).toBeNull()
  })

  it('ignores modified combinations outside the supported set', () => {
    const target = resolveSecurityEvidenceTransferShortcut({
      key: 'i',
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      shiftKey: true,
      defaultPrevented: false,
      repeat: false,
      isComposing: false,
      target: document.body,
    })

    expect(target).toBeNull()
  })
})
