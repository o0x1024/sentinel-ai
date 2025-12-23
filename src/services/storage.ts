/**
 * 持久化存储服务
 * 提供跨应用重启的数据存储能力
 */

export interface StorageOptions {
  /**
   * 是否使用内存缓存以提高性能
   */
  useMemoryCache?: boolean
}

class StorageService {
  private memoryCache: Map<string, any> = new Map()
  private useCache: boolean

  constructor(options: StorageOptions = {}) {
    this.useCache = options.useMemoryCache ?? true
  }

  /**
   * 设置存储项
   */
  async setItem<T = any>(key: string, value: T): Promise<void> {
    try {
      const serialized = JSON.stringify(value)
      localStorage.setItem(key, serialized)
      
      if (this.useCache) {
        this.memoryCache.set(key, value)
      }
    } catch (error) {
      console.error(`Failed to set storage item "${key}":`, error)
      throw error
    }
  }

  /**
   * 获取存储项
   */
  async getItem<T = any>(key: string): Promise<T | null> {
    try {
      // 先检查内存缓存
      if (this.useCache && this.memoryCache.has(key)) {
        return this.memoryCache.get(key) as T
      }

      const serialized = localStorage.getItem(key)
      if (serialized === null) {
        return null
      }

      const value = JSON.parse(serialized) as T
      
      // 更新内存缓存
      if (this.useCache) {
        this.memoryCache.set(key, value)
      }

      return value
    } catch (error) {
      console.error(`Failed to get storage item "${key}":`, error)
      return null
    }
  }

  /**
   * 移除存储项
   */
  async removeItem(key: string): Promise<void> {
    try {
      localStorage.removeItem(key)
      this.memoryCache.delete(key)
    } catch (error) {
      console.error(`Failed to remove storage item "${key}":`, error)
      throw error
    }
  }

  /**
   * 清空所有存储
   */
  async clear(): Promise<void> {
    try {
      localStorage.clear()
      this.memoryCache.clear()
    } catch (error) {
      console.error('Failed to clear storage:', error)
      throw error
    }
  }

  /**
   * 检查存储项是否存在
   */
  async hasItem(key: string): Promise<boolean> {
    try {
      return localStorage.getItem(key) !== null
    } catch (error) {
      console.error(`Failed to check storage item "${key}":`, error)
      return false
    }
  }

  /**
   * 获取所有键
   */
  async keys(): Promise<string[]> {
    try {
      return Object.keys(localStorage)
    } catch (error) {
      console.error('Failed to get storage keys:', error)
      return []
    }
  }

  /**
   * 获取存储大小（字节）
   */
  getSize(): number {
    try {
      let size = 0
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i)
        if (key) {
          const value = localStorage.getItem(key)
          if (value) {
            size += key.length + value.length
          }
        }
      }
      return size
    } catch (error) {
      console.error('Failed to get storage size:', error)
      return 0
    }
  }

  /**
   * 清理过期数据
   */
  async cleanupExpired(prefix: string, maxAge: number): Promise<number> {
    try {
      let cleaned = 0
      const now = Date.now()
      const keys = Object.keys(localStorage)

      for (const key of keys) {
        if (key.startsWith(prefix)) {
          const value = localStorage.getItem(key)
          if (value) {
            try {
              const data = JSON.parse(value)
              if (data.timestamp && now - data.timestamp > maxAge) {
                localStorage.removeItem(key)
                this.memoryCache.delete(key)
                cleaned++
              }
            } catch {
              // 忽略无法解析的数据
            }
          }
        }
      }

      return cleaned
    } catch (error) {
      console.error('Failed to cleanup expired storage:', error)
      return 0
    }
  }
}

// 导出单例实例
export const storage = new StorageService({ useMemoryCache: true })

// 导出类以便创建新实例
export { StorageService }

/**
 * 专门用于插件商店的缓存管理
 */
export interface PluginStoreCacheData {
  plugins: any[]
  timestamp: number
  version: string
}

export class PluginStoreCache {
  private static readonly CACHE_KEY = 'plugin_store_cache'
  private static readonly CACHE_DURATION = 5 * 60 * 1000 // 5分钟
  private static readonly VERSION = '1.0.0'

  /**
   * 保存插件列表到缓存
   */
  static async save(plugins: any[]): Promise<void> {
    const cacheData: PluginStoreCacheData = {
      plugins,
      timestamp: Date.now(),
      version: this.VERSION
    }
    await storage.setItem(this.CACHE_KEY, cacheData)
  }

  /**
   * 从缓存加载插件列表
   * @returns 如果缓存有效返回插件列表，否则返回 null
   */
  static async load(): Promise<{ plugins: any[]; timestamp: number } | null> {
    try {
      const cacheData = await storage.getItem<PluginStoreCacheData>(this.CACHE_KEY)
      
      if (!cacheData) {
        return null
      }

      // 检查版本
      if (cacheData.version !== this.VERSION) {
        console.log('Cache version mismatch, invalidating cache')
        await this.clear()
        return null
      }

      // 检查是否过期
      const now = Date.now()
      if (now - cacheData.timestamp > this.CACHE_DURATION) {
        console.log('Cache expired')
        return null
      }

      return {
        plugins: cacheData.plugins,
        timestamp: cacheData.timestamp
      }
    } catch (error) {
      console.error('Failed to load plugin store cache:', error)
      return null
    }
  }

  /**
   * 清除缓存
   */
  static async clear(): Promise<void> {
    await storage.removeItem(this.CACHE_KEY)
  }

  /**
   * 检查缓存是否存在且有效
   */
  static async isValid(): Promise<boolean> {
    const cache = await this.load()
    return cache !== null
  }

  /**
   * 获取缓存时间
   */
  static async getCacheTime(): Promise<number | null> {
    const cacheData = await storage.getItem<PluginStoreCacheData>(this.CACHE_KEY)
    return cacheData?.timestamp ?? null
  }
}

/**
 * 视图模式持久化
 */
export class ViewModeStorage {
  private static readonly KEY = 'plugin_store_view_mode'

  static async save(mode: 'list' | 'card'): Promise<void> {
    await storage.setItem(this.KEY, mode)
  }

  static async load(): Promise<'list' | 'card'> {
    const mode = await storage.getItem<'list' | 'card'>(this.KEY)
    return mode ?? 'list' // 默认列表视图
  }
}

/**
 * 初始化存储系统
 * 在应用启动时调用
 */
export async function initializeStorage(): Promise<void> {
  try {
    console.log('Initializing storage system...')
    
    // 检查存储是否可用
    const testKey = '__storage_test__'
    await storage.setItem(testKey, { test: true })
    await storage.removeItem(testKey)
    
    // 清理过期的缓存数据
    const cleaned = await storage.cleanupExpired('plugin_store', 24 * 60 * 60 * 1000) // 清理超过24小时的数据
    if (cleaned > 0) {
      console.log(`Cleaned up ${cleaned} expired cache entries`)
    }
    
    // 记录存储统计
    const size = storage.getSize()
    const keys = await storage.keys()
    console.log(`Storage initialized: ${keys.length} keys, ${(size / 1024).toFixed(2)} KB`)
    
  } catch (error) {
    console.error('Failed to initialize storage:', error)
  }
}

