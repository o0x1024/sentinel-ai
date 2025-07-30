import { describe, it, expect, beforeEach, vi } from 'vitest'
import { performanceService } from '@/services/performance'

// Mock Performance API
const mockPerformance = {
  mark: vi.fn(),
  measure: vi.fn(),
  now: vi.fn(() => 1000),
  memory: {
    usedJSHeapSize: 10 * 1024 * 1024,
    totalJSHeapSize: 20 * 1024 * 1024,
  },
}

// Mock PerformanceObserver
const MockPerformanceObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  disconnect: vi.fn(),
})) as any

MockPerformanceObserver.supportedEntryTypes = ['navigation', 'paint', 'largest-contentful-paint', 'first-input', 'layout-shift']

global.PerformanceObserver = MockPerformanceObserver

let service: typeof performanceService

describe('PerformanceService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    service = performanceService
    Object.assign(global.performance, mockPerformance)
  })

  describe('性能指标收集', () => {
    it('应该收集基础性能指标', () => {
      const metrics = service.getMetrics()
      expect(metrics).toBeDefined()
      expect(typeof metrics.memoryUsage).toBe('number')
      expect(typeof metrics.jsHeapSize).toBe('number')
    })

    it('应该正确计算内存使用量', () => {
      const metrics = service.getMetrics()
      expect(metrics.memoryUsage).toBe(10) // 10MB
      expect(metrics.jsHeapSize).toBe(20) // 20MB
    })
  })

  describe('路由性能监控', () => {
    it('应该标记路由开始时间', () => {
      service.markRouteStart('/dashboard')
      
      expect(mockPerformance.now).toHaveBeenCalled()
      expect(mockPerformance.mark).toHaveBeenCalledWith('route-start-/dashboard')
    })

    it('应该计算路由切换时间', () => {
      service.markRouteStart('/dashboard')
      service.markRouteEnd('/dashboard')
      
      const metrics = service.getMetrics()
      expect(metrics.routeChangeTime).toBeDefined()
      expect(mockPerformance.measure).toHaveBeenCalledWith(
        'route-/dashboard',
        'route-start-/dashboard',
        'route-end-/dashboard'
      )
    })
  })

  describe('API性能监控', () => {
    it('应该测量API调用时间', async () => {
      const mockApiCall = vi.fn().mockResolvedValue({ data: 'test' })
      
      const result = await service.measureApiCall(mockApiCall, 'test-api')
      
      expect(result).toEqual({ data: 'test' })
      expect(mockPerformance.mark).toHaveBeenCalledWith('api-start-test-api')
      expect(mockPerformance.mark).toHaveBeenCalledWith('api-end-test-api')
      expect(mockPerformance.measure).toHaveBeenCalledWith(
        'api-test-api',
        'api-start-test-api',
        'api-end-test-api'
      )
    })

    it('应该处理API调用错误', async () => {
      const mockApiCall = vi.fn().mockRejectedValue(new Error('API Error'))
      
      await expect(service.measureApiCall(mockApiCall, 'error-api')).rejects.toThrow('API Error')
      
      expect(mockPerformance.mark).toHaveBeenCalledWith('api-start-error-api')
      expect(mockPerformance.mark).toHaveBeenCalledWith('api-error-error-api')
      
      const metrics = service.getMetrics()
      expect(metrics.apiResponseTime).toBeDefined()
    })
  })

  describe('性能评分', () => {
    it('应该返回默认评分100', () => {
      const score = service.getPerformanceScore()
      expect(score).toBe(100)
    })

    it('应该根据LCP降低评分', () => {
      // 模拟设置LCP指标
      const metrics = service.getMetrics()
      metrics.largestContentfulPaint = 5000 // 大于4000ms
      
      // 手动设置指标以测试评分逻辑
      ;(service as any).metrics.largestContentfulPaint = 5000
      
      const score = service.getPerformanceScore()
      expect(score).toBeLessThan(100)
    })
  })

  describe('性能报告', () => {
    it('应该生成完整的性能报告', () => {
      const report = service.generateReport()
      
      expect(report).toContain('Sentinel AI 性能报告')
      expect(report).toContain('性能评分')
      expect(report).toContain('核心Web指标')
      expect(report).toContain('加载性能')
      expect(report).toContain('运行时性能')
      expect(report).toContain('用户体验')
    })

    it('应该包含性能建议', () => {
      const report = service.generateReport()
      
      // 基本检查报告包含相关内容
      expect(report).toContain('最大内容绘制')
      expect(report).toContain('首次输入延迟')
      expect(report).toContain('累积布局偏移')
    })
  })

  describe('资源清理', () => {
    it('应该正确销毁观察器', () => {
      const mockDisconnect = vi.fn()
      
      // 模拟观察器
      ;(service as any).observers = [
        { disconnect: mockDisconnect },
        { disconnect: mockDisconnect },
      ]
      
      service.destroy()
      
      expect(mockDisconnect).toHaveBeenCalledTimes(2)
      expect((service as any).observers).toHaveLength(0)
    })
  })
}) 