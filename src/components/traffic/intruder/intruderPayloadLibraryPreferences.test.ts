import { beforeEach, describe, expect, it } from 'vitest'
import {
  getIntruderPayloadLibraryPreferences,
  markIntruderPayloadLibraryRecent,
  toggleIntruderPayloadLibraryFavorite,
} from './intruderPayloadLibraryPreferences'

describe('intruder payload library preferences', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('toggles favorites in local storage', () => {
    let preferences = toggleIntruderPayloadLibraryFavorite('builtin:usernames')
    expect(preferences.favorites).toEqual(['builtin:usernames'])

    preferences = toggleIntruderPayloadLibraryFavorite('builtin:usernames')
    expect(preferences.favorites).toEqual([])
  })

  it('keeps recent sources unique and ordered', () => {
    markIntruderPayloadLibraryRecent('builtin:usernames')
    markIntruderPayloadLibraryRecent('builtin:passwords')
    const preferences = markIntruderPayloadLibraryRecent('builtin:usernames')

    expect(preferences.recent).toEqual(['builtin:usernames', 'builtin:passwords'])
    expect(getIntruderPayloadLibraryPreferences().recent).toEqual(['builtin:usernames', 'builtin:passwords'])
  })
})
