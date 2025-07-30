/**
 * 性能监控服务
 * 用于跟踪应用性能指标和用户体验
 */

export interface PerformanceMetrics {
  // 页面加载性能
  pageLoadTime: number;
  domContentLoadedTime: number;
  firstContentfulPaint: number;
  largestContentfulPaint: number;
  
  // 运行时性能
  memoryUsage: number;
  jsHeapSize: number;
  
  // 用户交互
  firstInputDelay: number;
  cumulativeLayoutShift: number;
  
  // 自定义指标
  routeChangeTime: number;
  apiResponseTime: number;
}

class PerformanceService {
  private metrics: Partial<PerformanceMetrics> = {};
  private observers: PerformanceObserver[] = [];
  private routeStartTime: number = 0;

  constructor() {
    this.initializeObservers();
    this.collectInitialMetrics();
  }

  /**
   * 初始化性能观察器
   */
  private initializeObservers() {
    if (!window.PerformanceObserver) return;

    // 观察导航时间
    const navObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      entries.forEach((entry) => {
        if (entry.entryType === 'navigation') {
          const navEntry = entry as PerformanceNavigationTiming;
          this.metrics.pageLoadTime = navEntry.loadEventEnd - navEntry.fetchStart;
          this.metrics.domContentLoadedTime = navEntry.domContentLoadedEventEnd - navEntry.fetchStart;
        }
      });
    });
    navObserver.observe({ entryTypes: ['navigation'] });
    this.observers.push(navObserver);

    // 观察绘制时间
    const paintObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      entries.forEach((entry) => {
        if (entry.name === 'first-contentful-paint') {
          this.metrics.firstContentfulPaint = entry.startTime;
        }
      });
    });
    paintObserver.observe({ entryTypes: ['paint'] });
    this.observers.push(paintObserver);

    // 观察最大内容绘制
    const lcpObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      const lastEntry = entries[entries.length - 1];
      this.metrics.largestContentfulPaint = lastEntry.startTime;
    });
    lcpObserver.observe({ entryTypes: ['largest-contentful-paint'] });
    this.observers.push(lcpObserver);

    // 观察首次输入延迟
    const fidObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      entries.forEach((entry: any) => {
        this.metrics.firstInputDelay = entry.processingStart - entry.startTime;
      });
    });
    fidObserver.observe({ entryTypes: ['first-input'] });
    this.observers.push(fidObserver);

    // 观察累积布局偏移
    const clsObserver = new PerformanceObserver((list) => {
      let clsValue = 0;
      const entries = list.getEntries();
      entries.forEach((entry: any) => {
        if (!entry.hadRecentInput) {
          clsValue += entry.value;
        }
      });
      this.metrics.cumulativeLayoutShift = clsValue;
    });
    clsObserver.observe({ entryTypes: ['layout-shift'] });
    this.observers.push(clsObserver);
  }

  /**
   * 收集初始性能指标
   */
  private collectInitialMetrics() {
    // 内存使用情况
    if ('memory' in performance) {
      const memInfo = (performance as any).memory;
      this.metrics.memoryUsage = memInfo.usedJSHeapSize / (1024 * 1024); // MB
      this.metrics.jsHeapSize = memInfo.totalJSHeapSize / (1024 * 1024); // MB
    }
  }

  /**
   * 标记路由开始时间
   */
  markRouteStart(routePath: string) {
    this.routeStartTime = performance.now();
    
    // 清除可能存在的旧标记，避免冲突
    try {
      performance.clearMarks(`route-start-${routePath}`);
      performance.clearMarks(`route-end-${routePath}`);
      performance.clearMeasures(`route-${routePath}`);
    } catch (error) {
      // 忽略清除失败的错误
    }
    
    try {
      performance.mark(`route-start-${routePath}`);
    } catch (error) {
      console.warn('Failed to create performance mark:', error);
    }
  }

  /**
   * 标记路由结束时间并计算耗时
   */
  markRouteEnd(routePath: string) {
    const endTime = performance.now();
    this.metrics.routeChangeTime = endTime - this.routeStartTime;
    
    try {
      performance.mark(`route-end-${routePath}`);
      
      // 尝试创建测量，如果失败则只记录时间差
      performance.measure(
        `route-${routePath}`,
        `route-start-${routePath}`,
        `route-end-${routePath}`
      );
    } catch (error) {
      // 如果测量失败，只记录警告但不影响功能
      console.warn('Failed to measure route performance:', error);
    }
  }

  /**
   * 测量API请求性能
   */
  async measureApiCall<T>(
    apiCall: () => Promise<T>,
    apiName: string
  ): Promise<T> {
    const startTime = performance.now();
    
    // 清除可能存在的旧标记
    try {
      performance.clearMarks(`api-start-${apiName}`);
      performance.clearMarks(`api-end-${apiName}`);
      performance.clearMarks(`api-error-${apiName}`);
      performance.clearMeasures(`api-${apiName}`);
    } catch (error) {
      // 忽略清除失败的错误
    }
    
    try {
      performance.mark(`api-start-${apiName}`);
    } catch (error) {
      console.warn('Failed to create API performance mark:', error);
    }
    
    try {
      const result = await apiCall();
      const endTime = performance.now();
      
      try {
        performance.mark(`api-end-${apiName}`);
        performance.measure(
          `api-${apiName}`,
          `api-start-${apiName}`,
          `api-end-${apiName}`
        );
      } catch (error) {
        console.warn('Failed to measure API performance:', error);
      }
      
      this.metrics.apiResponseTime = endTime - startTime;
      
      return result;
    } catch (error) {
      const endTime = performance.now();
      
      try {
        performance.mark(`api-error-${apiName}`);
      } catch (markError) {
        console.warn('Failed to create API error mark:', markError);
      }
      
      this.metrics.apiResponseTime = endTime - startTime;
      throw error;
    }
  }

  /**
   * 获取当前性能指标
   */
  getMetrics(): Partial<PerformanceMetrics> {
    return { ...this.metrics };
  }

  /**
   * 获取性能评分 (0-100)
   */
  getPerformanceScore(): number {
    let score = 100;
    
    // LCP评分 (0-40分)
    if (this.metrics.largestContentfulPaint) {
      if (this.metrics.largestContentfulPaint > 4000) score -= 40;
      else if (this.metrics.largestContentfulPaint > 2500) score -= 20;
    }
    
    // FID评分 (0-30分)
    if (this.metrics.firstInputDelay) {
      if (this.metrics.firstInputDelay > 300) score -= 30;
      else if (this.metrics.firstInputDelay > 100) score -= 15;
    }
    
    // CLS评分 (0-30分)
    if (this.metrics.cumulativeLayoutShift) {
      if (this.metrics.cumulativeLayoutShift > 0.25) score -= 30;
      else if (this.metrics.cumulativeLayoutShift > 0.1) score -= 15;
    }
    
    return Math.max(0, score);
  }

  /**
   * 生成性能报告
   */
  generateReport(): string {
    const metrics = this.getMetrics();
    const score = this.getPerformanceScore();
    
    return `
=== Sentinel AI 性能报告 ===
性能评分: ${score}/100

核心Web指标:
- 最大内容绘制 (LCP): ${metrics.largestContentfulPaint?.toFixed(2) || 'N/A'}ms
- 首次输入延迟 (FID): ${metrics.firstInputDelay?.toFixed(2) || 'N/A'}ms
- 累积布局偏移 (CLS): ${metrics.cumulativeLayoutShift?.toFixed(3) || 'N/A'}

加载性能:
- 页面加载时间: ${metrics.pageLoadTime?.toFixed(2) || 'N/A'}ms
- DOM内容加载: ${metrics.domContentLoadedTime?.toFixed(2) || 'N/A'}ms
- 首次内容绘制: ${metrics.firstContentfulPaint?.toFixed(2) || 'N/A'}ms

运行时性能:
- 内存使用: ${metrics.memoryUsage?.toFixed(2) || 'N/A'}MB
- JS堆大小: ${metrics.jsHeapSize?.toFixed(2) || 'N/A'}MB

用户体验:
- 路由切换时间: ${metrics.routeChangeTime?.toFixed(2) || 'N/A'}ms
- API响应时间: ${metrics.apiResponseTime?.toFixed(2) || 'N/A'}ms
    `;
  }

  /**
   * 清理观察器
   */
  destroy() {
    this.observers.forEach(observer => observer.disconnect());
    this.observers = [];
  }
}

// 创建单例实例
export const performanceService = new PerformanceService();

// 在开发环境下暴露到window对象以便调试
if (import.meta.env.DEV) {
  (window as any).performanceService = performanceService;
} 