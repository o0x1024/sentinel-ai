import { beforeEach, describe, expect, it } from 'vitest'
import {
  addCustomPinnedSearchShortcutTagPreset,
  loadCustomPinnedSearchShortcutTagPresets,
  renameCustomPinnedSearchShortcutTagPreset,
  reorderCustomPinnedSearchShortcutTagPresets,
  removeCustomPinnedSearchShortcutTagPreset,
  saveCustomPinnedSearchShortcutTagPresets,
} from '@/services/pinnedSearchShortcutTagPresetStorage'

describe('pinnedSearchShortcutTagPresetStorage', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('loads empty presets by default', () => {
    expect(loadCustomPinnedSearchShortcutTagPresets()).toEqual([])
  })

  it('adds custom presets with normalization', () => {
    expect(addCustomPinnedSearchShortcutTagPreset(' 专注 ')).toEqual(['专注'])
    expect(addCustomPinnedSearchShortcutTagPreset('专注')).toEqual(['专注'])
  })

  it('removes custom presets', () => {
    saveCustomPinnedSearchShortcutTagPresets(['专注', '夜班'])
    expect(removeCustomPinnedSearchShortcutTagPreset('专注')).toEqual(['夜班'])
  })

  it('renames custom presets with normalization', () => {
    saveCustomPinnedSearchShortcutTagPresets(['专注', '夜班'])
    expect(renameCustomPinnedSearchShortcutTagPreset('专注', ' 值班 ')).toEqual(['值班', '夜班'])
  })

  it('reorders custom presets', () => {
    saveCustomPinnedSearchShortcutTagPresets(['专注', '夜班', '漏洞'])
    expect(reorderCustomPinnedSearchShortcutTagPresets('漏洞', '专注')).toEqual(['漏洞', '专注', '夜班'])
  })
})
