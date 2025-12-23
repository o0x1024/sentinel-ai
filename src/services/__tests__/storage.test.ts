import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { 
  StorageService, 
  PluginStoreCache, 
  ViewModeStorage,
  initializeStorage 
} from '../storage'

describe('StorageService', () => {
  let storage: StorageService

  beforeEach(() => {
    storage = new StorageService()
    localStorage.clear()
  })

  afterEach(() => {
    localStorage.clear()
  })

  it('should store and retrieve items', async () => {
    await storage.setItem('test-key', { data: 'test-value' })
    const result = await storage.getItem('test-key')
    expect(result).toEqual({ data: 'test-value' })
  })

  it('should return null for non-existent items', async () => {
    const result = await storage.getItem('non-existent')
    expect(result).toBeNull()
  })

  it('should remove items', async () => {
    await storage.setItem('test-key', 'value')
    await storage.removeItem('test-key')
    const result = await storage.getItem('test-key')
    expect(result).toBeNull()
  })

  it('should check if items exist', async () => {
    await storage.setItem('test-key', 'value')
    expect(await storage.hasItem('test-key')).toBe(true)
    expect(await storage.hasItem('non-existent')).toBe(false)
  })

  it('should get all keys', async () => {
    await storage.setItem('key1', 'value1')
    await storage.setItem('key2', 'value2')
    const keys = await storage.keys()
    expect(keys).toContain('key1')
    expect(keys).toContain('key2')
  })

  it('should clear all storage', async () => {
    await storage.setItem('key1', 'value1')
    await storage.setItem('key2', 'value2')
    await storage.clear()
    const keys = await storage.keys()
    expect(keys).toHaveLength(0)
  })
})

describe('PluginStoreCache', () => {
  beforeEach(async () => {
    localStorage.clear()
    // 确保清除所有缓存
    await PluginStoreCache.clear()
  })

  afterEach(async () => {
    localStorage.clear()
    await PluginStoreCache.clear()
  })

  it('should save and load plugin cache', async () => {
    const plugins = [
      { id: 'plugin1', name: 'Plugin 1' },
      { id: 'plugin2', name: 'Plugin 2' }
    ]

    await PluginStoreCache.save(plugins)
    const result = await PluginStoreCache.load()

    expect(result).not.toBeNull()
    expect(result?.plugins).toEqual(plugins)
    expect(result?.timestamp).toBeGreaterThan(0)
  })

  it('should return null for expired cache', async () => {
    const plugins = [{ id: 'plugin1', name: 'Plugin 1' }]
    
    // Save cache with old timestamp
    const oldTimestamp = Date.now() - 10 * 60 * 1000 // 10 minutes ago
    localStorage.setItem('plugin_store_cache', JSON.stringify({
      plugins,
      timestamp: oldTimestamp,
      version: '1.0.0'
    }))

    const result = await PluginStoreCache.load()
    expect(result).toBeNull()
  })

  it('should return null for version mismatch', async () => {
    const plugins = [{ id: 'plugin1', name: 'Plugin 1' }]
    
    // Save cache with different version
    localStorage.setItem('plugin_store_cache', JSON.stringify({
      plugins,
      timestamp: Date.now(),
      version: '0.0.1'
    }))

    const result = await PluginStoreCache.load()
    expect(result).toBeNull()
  })

  it('should check if cache is valid', async () => {
    expect(await PluginStoreCache.isValid()).toBe(false)

    const plugins = [{ id: 'plugin1', name: 'Plugin 1' }]
    await PluginStoreCache.save(plugins)

    expect(await PluginStoreCache.isValid()).toBe(true)
  })

  it('should get cache timestamp', async () => {
    const plugins = [{ id: 'plugin1', name: 'Plugin 1' }]
    await PluginStoreCache.save(plugins)

    const timestamp = await PluginStoreCache.getCacheTime()
    expect(timestamp).toBeGreaterThan(0)
  })

  it('should clear cache', async () => {
    const plugins = [{ id: 'plugin1', name: 'Plugin 1' }]
    await PluginStoreCache.save(plugins)

    await PluginStoreCache.clear()
    expect(await PluginStoreCache.isValid()).toBe(false)
  })
})

describe('ViewModeStorage', () => {
  beforeEach(async () => {
    localStorage.clear()
    // 清除视图模式设置
    localStorage.removeItem('plugin_store_view_mode')
  })

  afterEach(async () => {
    localStorage.clear()
  })

  it('should return default list view when not set', async () => {
    const result = await ViewModeStorage.load()
    expect(result).toBe('list')
  })

  it('should save and load view mode', async () => {
    await ViewModeStorage.save('card')
    const result = await ViewModeStorage.load()
    expect(result).toBe('card')
  })

  it('should persist view mode across reloads', async () => {
    await ViewModeStorage.save('card')
    
    // Simulate reload by creating new storage instance
    const result = await ViewModeStorage.load()
    expect(result).toBe('card')
  })
})

describe('initializeStorage', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  afterEach(() => {
    localStorage.clear()
  })

  it('should initialize storage without errors', async () => {
    await expect(initializeStorage()).resolves.not.toThrow()
  })

  it('should clean up expired data', async () => {
    // Add some old data
    const oldTimestamp = Date.now() - 25 * 60 * 60 * 1000 // 25 hours ago
    localStorage.setItem('plugin_store_old', JSON.stringify({
      data: 'old',
      timestamp: oldTimestamp
    }))

    await initializeStorage()

    // Old data should be removed
    expect(localStorage.getItem('plugin_store_old')).toBeNull()
  })
})

