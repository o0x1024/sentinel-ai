import { beforeEach, describe, expect, it } from 'vitest'
import type { Column, ProxyRequest } from './proxyHistoryTypes'
import {
  defaultProxyHistoryColumns,
  getProxyHistoryRequestPath,
  loadProxyHistoryColumnsFromStorage,
  moveProxyHistoryColumn,
  PROXY_HISTORY_COLUMNS_STORAGE_KEY,
} from './proxyHistoryTableSupport'

describe('proxyHistoryTableSupport', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('places path at the primary URL position', () => {
    expect(defaultProxyHistoryColumns.slice(0, 6).map(column => column.id)).toEqual([
      'id',
      'path',
      'host',
      'method',
      'httpVersion',
      'url',
    ])
  })

  it('extracts request path with query from full URL', () => {
    expect(
      getProxyHistoryRequestPath({
        url: 'https://example.com/api/items?page=1',
      } as ProxyRequest),
    ).toBe('/api/items?page=1')
  })

  it('moves a column before the target column', () => {
    const columns = [
      { id: 'id' },
      { id: 'path' },
      { id: 'host' },
    ] as Column[]

    expect(moveProxyHistoryColumn(columns, 'host', 'path', 'before').map(column => column.id)).toEqual([
      'id',
      'host',
      'path',
    ])
  })

  it('moves a column after the target column', () => {
    const columns = [
      { id: 'id' },
      { id: 'path' },
      { id: 'host' },
    ] as Column[]

    expect(moveProxyHistoryColumn(columns, 'id', 'host', 'after').map(column => column.id)).toEqual([
      'path',
      'host',
      'id',
    ])
  })

  it('loads stored columns in the stored order', () => {
    const savedColumns = [
      defaultProxyHistoryColumns.find(column => column.id === 'host'),
      defaultProxyHistoryColumns.find(column => column.id === 'path'),
      defaultProxyHistoryColumns.find(column => column.id === 'id'),
    ]
    localStorage.setItem(PROXY_HISTORY_COLUMNS_STORAGE_KEY, JSON.stringify(savedColumns))

    expect(loadProxyHistoryColumnsFromStorage().slice(0, 3).map(column => column.id)).toEqual([
      'host',
      'path',
      'id',
    ])
  })
})
