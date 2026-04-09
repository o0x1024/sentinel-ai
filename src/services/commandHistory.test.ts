import { beforeEach, describe, expect, it } from 'vitest'
import { addRecentCommand, clearRecentCommands, loadRecentCommands, saveRecentCommands } from '@/services/commandHistory'

describe('commandHistory', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('stores newest commands first', () => {
    addRecentCommand('theme set dark')
    addRecentCommand('finding severity:critical')
    expect(loadRecentCommands()).toEqual(['finding severity:critical', 'theme set dark'])
  })

  it('deduplicates commands case-insensitively', () => {
    saveRecentCommands(['theme set dark', 'notify open rules'])
    expect(addRecentCommand('Theme Set Dark')).toEqual(['Theme Set Dark', 'notify open rules'])
  })

  it('clears saved commands', () => {
    saveRecentCommands(['theme set dark', 'notify open rules'])
    expect(clearRecentCommands()).toEqual([])
    expect(loadRecentCommands()).toEqual([])
  })
})
