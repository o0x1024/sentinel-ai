import { describe, expect, it, vi } from 'vitest'
import { setLocalStorageItem, setSessionStorageItem } from './browserStorage'

describe('browserStorage', () => {
  it('does not throw when localStorage is full', () => {
    const spy = vi.spyOn(window.localStorage.__proto__, 'setItem').mockImplementation(() => {
      throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
    })

    expect(() => setLocalStorageItem('key', 'value')).not.toThrow()
    spy.mockRestore()
  })

  it('does not throw when sessionStorage is full', () => {
    const spy = vi.spyOn(window.sessionStorage.__proto__, 'setItem').mockImplementation(() => {
      throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
    })

    expect(() => setSessionStorageItem('key', 'value')).not.toThrow()
    spy.mockRestore()
  })
})
