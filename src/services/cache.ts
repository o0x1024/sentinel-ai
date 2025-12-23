/**
 * 后端缓存服务
 * 使用 Tauri 后端数据库存储缓存数据
 */

import { invoke } from '@tauri-apps/api/core'

export interface SetCacheRequest {
  key: string
  value: string
  cache_type: string
  ttl_minutes?: number
}

export interface GetCacheResponse {
  success: boolean
  data?: string
  error?: string
}

export interface SetCacheResponse {
  success: boolean
  error?: string
}

/**
 * 缓存服务类
 */
export class CacheService {
  /**
   * 获取缓存
   */
  static async get<T = any>(key: string): Promise<T | null> {
    try {
      const response = await invoke<GetCacheResponse>('get_cache', { key })
      
      if (!response.success || !response.data) {
        return null
      }

      return JSON.parse(response.data) as T
    } catch (error) {
      console.error(`Failed to get cache for key "${key}":`, error)
      return null
    }
  }

  /**
   * 设置缓存
   */
  static async set<T = any>(
    key: string, 
    value: T, 
    cacheType: string,
    ttlMinutes?: number
  ): Promise<boolean> {
    try {
      const request: SetCacheRequest = {
        key,
        value: JSON.stringify(value),
        cache_type: cacheType,
        ttl_minutes: ttlMinutes
      }

      const response = await invoke<SetCacheResponse>('set_cache', { request })
      return response.success
    } catch (error) {
      console.error(`Failed to set cache for key "${key}":`, error)
      return false
    }
  }

  /**
   * 删除缓存
   */
  static async delete(key: string): Promise<boolean> {
    try {
      const response = await invoke<SetCacheResponse>('delete_cache', { key })
      return response.success
    } catch (error) {
      console.error(`Failed to delete cache for key "${key}":`, error)
      return false
    }
  }

  /**
   * 清理过期缓存
   */
  static async cleanupExpired(): Promise<boolean> {
    try {
      const response = await invoke<SetCacheResponse>('cleanup_expired_cache')
      return response.success
    } catch (error) {
      console.error('Failed to cleanup expired cache:', error)
      return false
    }
  }

  /**
   * 获取所有缓存键
   */
  static async getAllKeys(cacheType?: string): Promise<string[]> {
    try {
      return await invoke<string[]>('get_all_cache_keys', { cacheType })
    } catch (error) {
      console.error('Failed to get all cache keys:', error)
      return []
    }
  }
}

/**
 * 插件商店缓存管理
 */
export interface PluginStoreCacheData {
  plugins: any[]
  timestamp: number
}

export class PluginStoreCache {
  private static readonly CACHE_KEY = 'plugin_store_cache'
  private static readonly CACHE_TYPE = 'plugin_store'
  private static readonly CACHE_TTL_MINUTES = 5 // 5分钟

  /**
   * 保存插件列表到缓存
   */
  static async save(plugins: any[]): Promise<void> {
    const cacheData: PluginStoreCacheData = {
      plugins,
      timestamp: Date.now()
    }

    await CacheService.set(
      this.CACHE_KEY,
      cacheData,
      this.CACHE_TYPE,
      this.CACHE_TTL_MINUTES
    )
  }

  /**
   * 从缓存加载插件列表
   */
  static async load(): Promise<PluginStoreCacheData | null> {
    return await CacheService.get<PluginStoreCacheData>(this.CACHE_KEY)
  }

  /**
   * 清除缓存
   */
  static async clear(): Promise<void> {
    await CacheService.delete(this.CACHE_KEY)
  }
}

/**
 * 视图模式持久化
 */
export class ViewModeStorage {
  private static readonly CACHE_KEY = 'plugin_store_view_mode'
  private static readonly CACHE_TYPE = 'view_mode'

  /**
   * 保存视图模式（永久有效）
   */
  static async save(mode: 'list' | 'card'): Promise<void> {
    await CacheService.set(
      this.CACHE_KEY,
      mode,
      this.CACHE_TYPE
      // 不设置 ttl，永久有效
    )
  }

  /**
   * 加载视图模式
   */
  static async load(): Promise<'list' | 'card'> {
    const mode = await CacheService.get<'list' | 'card'>(this.CACHE_KEY)
    return mode ?? 'list' // 默认列表视图
  }
}

/**
 * 初始化缓存系统
 */
export async function initializeCache(): Promise<void> {
  try {
    console.log('Initializing cache system...')
    
    // 清理过期缓存
    const success = await CacheService.cleanupExpired()
    if (success) {
      console.log('Cache system initialized and expired entries cleaned')
    }
  } catch (error) {
    console.error('Failed to initialize cache:', error)
  }
}

