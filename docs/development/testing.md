# 测试指南

本指南介绍如何在 Sentinel AI 项目中编写和运行测试。

## 测试架构

Sentinel AI 采用多层次的测试策略：

### 测试分层
```
E2E Tests (端到端测试)
    ↑
Integration Tests (集成测试)  
    ↑
Unit Tests (单元测试)
```

### 测试工具
- **Vitest**: 快速的单元测试框架
- **Vue Test Utils**: Vue组件测试工具
- **Playwright**: 端到端测试框架
- **JSDOM**: DOM环境模拟

## 单元测试

### 组件测试

Vue组件的单元测试示例：

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import StatsCard from '@/components/StatsCard.vue'

describe('StatsCard.vue', () => {
  it('renders properly with required props', () => {
    const wrapper = mount(StatsCard, {
      props: {
        title: 'Test Title',
        value: '123',
        icon: 'fas fa-test',
        theme: 'primary' as const
      }
    })

    expect(wrapper.text()).toContain('Test Title')
    expect(wrapper.text()).toContain('123')
  })

  it('emits click event when clicked', async () => {
    const wrapper = mount(StatsCard, {
      props: {
        title: 'Test',
        value: '100',
        icon: 'fas fa-chart',
        theme: 'primary' as const
      }
    })

    await wrapper.trigger('click')
    expect(wrapper.emitted()).toHaveProperty('click')
  })
})
```

### 服务测试

Service类的单元测试示例：

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { PerformanceService } from '@/services/performance'

describe('PerformanceService', () => {
  let performanceService: PerformanceService

  beforeEach(() => {
    vi.clearAllMocks()
    performanceService = new PerformanceService()
  })

  it('should track page load time', () => {
    performanceService.markPageLoadStart()
    vi.advanceTimersByTime(1000)
    performanceService.markPageLoadEnd()
    
    const metrics = performanceService.getMetrics()
    expect(metrics.pageLoadTime).toBeGreaterThan(0)
  })
})
```

### Mock配置

测试中的Mock配置在 `src/tests/setup.ts` 中定义：

```typescript
import { vi } from 'vitest'
import { config } from '@vue/test-utils'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock Chart.js
vi.mock('chart.js', () => ({
  Chart: vi.fn().mockImplementation(() => ({
    destroy: vi.fn(),
    update: vi.fn(),
  })),
  registerables: [],
}))
```

## 集成测试

集成测试验证模块间的交互：

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

describe('API Integration Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should create scan task successfully', async () => {
    const mockTask = {
      id: '1',
      name: 'Test Scan',
      target: 'example.com',
      status: 'pending',
    }

    vi.mocked(invoke).mockResolvedValueOnce(mockTask)

    const result = await invoke('create_scan_task', {
      name: 'Test Scan',
      target: 'example.com',
      scan_type: 'comprehensive',
    })

    expect(result).toEqual(mockTask)
  })
})
```

## E2E测试

端到端测试验证完整的用户流程：

```typescript
import { test, expect } from '@playwright/test'

test.describe('Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('should display main dashboard elements', async ({ page }) => {
    await expect(page).toHaveTitle(/Sentinel AI/)
    await expect(page.locator('[data-testid="navbar"]')).toBeVisible()
    await expect(page.locator('[data-testid="stats-grid"]')).toBeVisible()
  })

  test('should navigate between pages', async ({ page }) => {
    await page.click('[data-testid="nav-scan-tasks"]')
    await expect(page.url()).toContain('/scan-tasks')
  })
})
```

## 运行测试

### 本地开发

```bash
# 运行所有单元测试
npm run test

# 运行测试并生成覆盖率报告
npm run test:coverage

# 观察模式运行测试
npm run test:watch

# 运行测试UI界面
npm run test:ui
```

### E2E测试

```bash
# 运行E2E测试
npm run test:e2e

# 运行E2E测试UI界面
npm run test:e2e:ui

# 调试模式运行E2E测试
npx playwright test --debug
```

### CI/CD环境

在GitHub Actions中的测试配置：

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
      
      - name: Install dependencies
        run: npm ci
      
      - name: Run unit tests
        run: npm run test:coverage
      
      - name: Run E2E tests
        run: npm run test:e2e
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

## 测试数据

### 测试用例数据

创建测试数据工厂：

```typescript
// src/tests/factories/scanTask.ts
export const createMockScanTask = (overrides = {}) => ({
  id: '1',
  name: 'Test Scan',
  target: 'example.com',
  status: 'pending',
  created_at: new Date().toISOString(),
  ...overrides,
})

export const createMockVulnerability = (overrides = {}) => ({
  id: '1',
  title: 'XSS Vulnerability',
  severity: 'high',
  status: 'open',
  url: 'https://example.com',
  ...overrides,
})
```

### 数据库测试

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import { Database } from '@/services/database'

describe('Database Operations', () => {
  let db: Database

  beforeEach(async () => {
    db = new Database(':memory:')
    await db.migrate()
  })

  it('should create and retrieve scan task', async () => {
    const task = await db.createScanTask({
      name: 'Test Scan',
      target: 'example.com',
    })

    expect(task.id).toBeDefined()
    expect(task.name).toBe('Test Scan')

    const retrieved = await db.getScanTask(task.id)
    expect(retrieved).toEqual(task)
  })
})
```

## 性能测试

### 前端性能测试

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import VirtualList from '@/components/VirtualList.vue'

describe('VirtualList Performance', () => {
  it('should handle large datasets efficiently', async () => {
    const start = performance.now()
    
    const items = Array.from({ length: 10000 }, (_, i) => ({
      id: i,
      name: `Item ${i}`,
    }))

    const wrapper = mount(VirtualList, {
      props: {
        items,
        itemHeight: 50,
        containerHeight: 400,
      }
    })

    const end = performance.now()
    const renderTime = end - start

    expect(renderTime).toBeLessThan(100) // 渲染时间小于100ms
    expect(wrapper.findAll('.virtual-item')).toHaveLength(8) // 只渲染可见项
  })
})
```

### 后端性能测试

Rust后端的性能测试：

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_scan_task_creation_performance() {
        let db = Database::new(":memory:").await.unwrap();
        
        let start = Instant::now();
        
        for i in 0..1000 {
            let task = CreateScanTaskRequest {
                name: format!("Task {}", i),
                target: format!("example{}.com", i),
                scan_type: "basic".to_string(),
            };
            
            db.create_scan_task(task).await.unwrap();
        }
        
        let duration = start.elapsed();
        assert!(duration.as_millis() < 1000); // 1000个任务创建时间小于1秒
    }
}
```

## 代码覆盖率

### 覆盖率目标

项目的代码覆盖率目标：
- **行覆盖率**: ≥ 80%
- **分支覆盖率**: ≥ 80%  
- **函数覆盖率**: ≥ 80%
- **语句覆盖率**: ≥ 80%

### 覆盖率报告

```bash
# 生成覆盖率报告
npm run test:coverage

# 查看详细报告
open coverage/index.html
```

### 覆盖率配置

在 `vitest.config.ts` 中配置：

```typescript
export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      thresholds: {
        global: {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80,
        },
      },
    },
  },
})
```

## 测试最佳实践

### 1. 测试命名

使用描述性的测试名称：

```typescript
// ✅ 好的命名
it('should display error message when API request fails')

// ❌ 不好的命名  
it('test error')
```

### 2. 测试隔离

每个测试应该独立运行：

```typescript
describe('UserService', () => {
  beforeEach(() => {
    // 重置状态
    vi.clearAllMocks()
  })
})
```

### 3. 异步测试

正确处理异步操作：

```typescript
it('should handle async operations', async () => {
  const promise = service.getData()
  await expect(promise).resolves.toBeDefined()
})
```

### 4. 边界条件测试

测试边界情况和错误状态：

```typescript
it('should handle empty input', () => {
  expect(() => processData([])).not.toThrow()
})

it('should throw error for invalid input', () => {
  expect(() => processData(null)).toThrow()
})
```

## 调试测试

### VSCode调试配置

`.vscode/launch.json`:

```json
{
  "type": "node",
  "request": "launch",
  "name": "Debug Vitest Tests",
  "runtimeExecutable": "npm",
  "runtimeArgs": ["run", "test:debug"],
  "console": "integratedTerminal",
  "internalConsoleOptions": "neverOpen"
}
```

### 调试技巧

```typescript
import { vi } from 'vitest'

it('should debug test', () => {
  // 使用console.log进行调试
  console.log('Debug info:', data)
  
  // 使用vi.spyOn监控函数调用
  const spy = vi.spyOn(service, 'method')
  
  // 检查调用参数
  expect(spy).toHaveBeenCalledWith(expectedArgs)
})
```

## 持续集成

测试在CI/CD流水线中自动运行，确保代码质量：

1. **代码提交时**：运行单元测试和集成测试
2. **Pull Request时**：运行完整测试套件
3. **发布前**：运行所有测试包括E2E测试
4. **定期任务**：运行性能回归测试

通过完善的测试体系，确保 Sentinel AI 的代码质量和稳定性。 