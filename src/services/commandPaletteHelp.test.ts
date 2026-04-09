import { describe, expect, it } from 'vitest'
import { getCommandPaletteHelpItems, isCommandHelpQuery } from '@/services/commandPaletteHelp'

describe('commandPaletteHelp', () => {
  it('detects help queries', () => {
    expect(isCommandHelpQuery('?')).toBe(true)
    expect(isCommandHelpQuery('help')).toBe(true)
    expect(isCommandHelpQuery('commands finding')).toBe(true)
    expect(isCommandHelpQuery('finding severity:critical')).toBe(false)
  })

  it('returns all help items for bare help queries', () => {
    expect(getCommandPaletteHelpItems('?').map(item => item.id)).toEqual([
      'theme',
      'finding',
      'notification',
    ])
  })

  it('filters help items by topic', () => {
    expect(getCommandPaletteHelpItems('? finding').map(item => item.id)).toEqual(['finding'])
    expect(getCommandPaletteHelpItems('help notify').map(item => item.id)).toEqual(['notification'])
  })
})
