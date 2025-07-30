import { test, expect } from '@playwright/test'

test.describe('Scan Sessions Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/scan-sessions')
  })

  test('should display scan sessions page elements', async ({ page }) => {
    // Check page title and description
    await expect(page.locator('h1')).toContainText('智能扫描会话')
    await expect(page.locator('.text-gray-600')).toContainText('管理和监控智能扫描会话')

    // Check action buttons
    await expect(page.locator('[data-testid="new-session-btn"]')).toBeVisible()
    await expect(page.locator('[data-testid="refresh-btn"]')).toBeVisible()

    // Check search input
    await expect(page.locator('[data-testid="search-input"]')).toBeVisible()

    // Check sessions table
    await expect(page.locator('[data-testid="sessions-table"]')).toBeVisible()
  })

  test('should open create session modal', async ({ page }) => {
    // Click new session button
    await page.click('[data-testid="new-session-btn"]')

    // Check modal is visible
    await expect(page.locator('[data-testid="create-session-modal"]')).toBeVisible()
    await expect(page.locator('.modal-title')).toContainText('创建新扫描会话')

    // Check form fields
    await expect(page.locator('[data-testid="session-name-input"]')).toBeVisible()
    await expect(page.locator('[data-testid="target-url-input"]')).toBeVisible()
    await expect(page.locator('[data-testid="scan-type-select"]')).toBeVisible()
    await expect(page.locator('[data-testid="scan-depth-select"]')).toBeVisible()
    await expect(page.locator('[data-testid="max-concurrency-input"]')).toBeVisible()
    await expect(page.locator('[data-testid="enable-ai-checkbox"]')).toBeVisible()
    await expect(page.locator('[data-testid="auto-optimize-checkbox"]')).toBeVisible()

    // Check action buttons
    await expect(page.locator('[data-testid="cancel-btn"]')).toBeVisible()
    await expect(page.locator('[data-testid="create-btn"]')).toBeVisible()
  })

  test('should create new scan session', async ({ page }) => {
    // Mock the create session API call
    await page.route('**/create_scan_session', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'session-test-1',
          name: 'Test Session',
          target_url: 'https://example.com',
          status: 'created',
          progress: 0,
          created_at: new Date().toISOString(),
        }),
      })
    })

    // Open create session modal
    await page.click('[data-testid="new-session-btn"]')

    // Fill form
    await page.fill('[data-testid="session-name-input"]', 'Test Session')
    await page.fill('[data-testid="target-url-input"]', 'https://example.com')
    await page.selectOption('[data-testid="scan-type-select"]', 'comprehensive')
    await page.selectOption('[data-testid="scan-depth-select"]', 'medium')
    await page.fill('[data-testid="max-concurrency-input"]', '10')
    await page.check('[data-testid="enable-ai-checkbox"]')
    await page.check('[data-testid="auto-optimize-checkbox"]')

    // Submit form
    await page.click('[data-testid="create-btn"]')

    // Check modal closes
    await expect(page.locator('[data-testid="create-session-modal"]')).not.toBeVisible()

    // Check success message
    await expect(page.locator('.toast-success')).toContainText('会话创建成功')
  })

  test('should validate form inputs', async ({ page }) => {
    // Open create session modal
    await page.click('[data-testid="new-session-btn"]')

    // Try to submit empty form
    await page.click('[data-testid="create-btn"]')

    // Check validation errors
    await expect(page.locator('.error-message')).toContainText('请输入会话名称')
    await expect(page.locator('.error-message')).toContainText('请输入目标URL')

    // Fill invalid URL
    await page.fill('[data-testid="target-url-input"]', 'invalid-url')
    await page.click('[data-testid="create-btn"]')
    await expect(page.locator('.error-message')).toContainText('请输入有效的URL')

    // Fill valid data
    await page.fill('[data-testid="session-name-input"]', 'Valid Session')
    await page.fill('[data-testid="target-url-input"]', 'https://example.com')

    // Check validation passes
    await expect(page.locator('.error-message')).not.toBeVisible()
  })

  test('should display session list', async ({ page }) => {
    // Mock sessions data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Test Session 1',
            target_url: 'https://example.com',
            status: 'running',
            progress: 45,
            current_stage: 'port_scan',
            created_at: new Date().toISOString(),
          },
          {
            id: 'session-2',
            name: 'Test Session 2',
            target_url: 'https://test.com',
            status: 'completed',
            progress: 100,
            current_stage: 'completed',
            created_at: new Date().toISOString(),
          },
        ]),
      })
    })

    // Refresh sessions
    await page.click('[data-testid="refresh-btn"]')

    // Check sessions are displayed
    await expect(page.locator('[data-testid="session-row"]')).toHaveCount(2)
    await expect(page.locator('[data-testid="session-row"]').first()).toContainText('Test Session 1')
    await expect(page.locator('[data-testid="session-row"]').first()).toContainText('running')
    await expect(page.locator('[data-testid="session-row"]').first()).toContainText('45%')
  })

  test('should filter sessions by search', async ({ page }) => {
    // Mock sessions data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Production Scan',
            target_url: 'https://prod.example.com',
            status: 'running',
          },
          {
            id: 'session-2',
            name: 'Test Environment',
            target_url: 'https://test.example.com',
            status: 'completed',
          },
        ]),
      })
    })

    // Wait for sessions to load
    await page.waitForSelector('[data-testid="session-row"]')

    // Search for "Production"
    await page.fill('[data-testid="search-input"]', 'Production')

    // Check filtered results
    await expect(page.locator('[data-testid="session-row"]')).toHaveCount(1)
    await expect(page.locator('[data-testid="session-row"]')).toContainText('Production Scan')

    // Clear search
    await page.fill('[data-testid="search-input"]', '')
    await expect(page.locator('[data-testid="session-row"]')).toHaveCount(2)
  })

  test('should control session operations', async ({ page }) => {
    // Mock session data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Test Session',
            target_url: 'https://example.com',
            status: 'created',
            progress: 0,
          },
        ]),
      })
    })

    // Mock control operations
    await page.route('**/start_scan_session', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'Session started' }),
      })
    })

    await page.route('**/pause_scan_session', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'Session paused' }),
      })
    })

    await page.route('**/stop_scan_session', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'Session stopped' }),
      })
    })

    // Wait for session to load
    await page.waitForSelector('[data-testid="session-row"]')

    // Test start session
    await page.click('[data-testid="start-session-btn"]')
    await expect(page.locator('.toast-success')).toContainText('会话已启动')

    // Test pause session
    await page.click('[data-testid="pause-session-btn"]')
    await expect(page.locator('.toast-success')).toContainText('会话已暂停')

    // Test stop session
    await page.click('[data-testid="stop-session-btn"]')
    await expect(page.locator('.toast-success')).toContainText('会话已停止')
  })

  test('should show session details modal', async ({ page }) => {
    // Mock session data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Test Session',
            target_url: 'https://example.com',
            status: 'running',
            progress: 65,
            current_stage: 'vulnerability_scan',
            scan_type: 'comprehensive',
            scan_depth: 'medium',
            max_concurrency: 10,
            enable_ai: true,
            auto_optimize: true,
            created_at: new Date().toISOString(),
            estimated_time: 1800,
            discovered_assets: {
              domains: 15,
              ips: 8,
              ports: 42,
              services: 23,
            },
          },
        ]),
      })
    })

    // Wait for session to load
    await page.waitForSelector('[data-testid="session-row"]')

    // Click view details button
    await page.click('[data-testid="view-details-btn"]')

    // Check details modal is visible
    await expect(page.locator('[data-testid="session-details-modal"]')).toBeVisible()
    await expect(page.locator('.modal-title')).toContainText('会话详情')

    // Check basic information
    await expect(page.locator('[data-testid="session-name"]')).toContainText('Test Session')
    await expect(page.locator('[data-testid="target-url"]')).toContainText('https://example.com')
    await expect(page.locator('[data-testid="session-status"]')).toContainText('运行中')

    // Check configuration
    await expect(page.locator('[data-testid="scan-type"]')).toContainText('全面扫描')
    await expect(page.locator('[data-testid="scan-depth"]')).toContainText('中等')
    await expect(page.locator('[data-testid="max-concurrency"]')).toContainText('10')

    // Check discovered assets
    await expect(page.locator('[data-testid="domains-count"]')).toContainText('15')
    await expect(page.locator('[data-testid="ips-count"]')).toContainText('8')
    await expect(page.locator('[data-testid="ports-count"]')).toContainText('42')
    await expect(page.locator('[data-testid="services-count"]')).toContainText('23')

    // Check scan stages
    await expect(page.locator('[data-testid="scan-stages"]')).toBeVisible()
    await expect(page.locator('[data-testid="current-stage"]')).toContainText('漏洞扫描')
  })

  test('should handle delete session', async ({ page }) => {
    // Mock session data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Test Session',
            target_url: 'https://example.com',
            status: 'completed',
          },
        ]),
      })
    })

    // Mock delete operation
    await page.route('**/delete_scan_session', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, message: 'Session deleted' }),
      })
    })

    // Wait for session to load
    await page.waitForSelector('[data-testid="session-row"]')

    // Click delete button
    await page.click('[data-testid="delete-session-btn"]')

    // Check confirmation dialog
    await expect(page.locator('[data-testid="confirm-dialog"]')).toBeVisible()
    await expect(page.locator('.dialog-message')).toContainText('确定要删除此会话吗')

    // Confirm deletion
    await page.click('[data-testid="confirm-delete-btn"]')

    // Check success message
    await expect(page.locator('.toast-success')).toContainText('会话已删除')
  })

  test('should export session report', async ({ page }) => {
    // Mock session data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Test Session',
            target_url: 'https://example.com',
            status: 'completed',
          },
        ]),
      })
    })

    // Mock export operation
    await page.route('**/export_session_report', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, file_path: '/tmp/report.pdf' }),
      })
    })

    // Wait for session to load
    await page.waitForSelector('[data-testid="session-row"]')

    // Click export button
    await page.click('[data-testid="export-report-btn"]')

    // Check success message
    await expect(page.locator('.toast-success')).toContainText('报告导出成功')
  })

  test('should clone session', async ({ page }) => {
    // Mock session data
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'session-1',
            name: 'Original Session',
            target_url: 'https://example.com',
            status: 'completed',
          },
        ]),
      })
    })

    // Mock clone operation
    await page.route('**/clone_scan_session', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'session-2',
          name: 'Original Session (副本)',
          target_url: 'https://example.com',
          status: 'created',
        }),
      })
    })

    // Wait for session to load
    await page.waitForSelector('[data-testid="session-row"]')

    // Click clone button
    await page.click('[data-testid="clone-session-btn"]')

    // Check success message
    await expect(page.locator('.toast-success')).toContainText('会话克隆成功')
  })

  test('should handle error states', async ({ page }) => {
    // Mock API error
    await page.route('**/list_scan_sessions', async route => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      })
    })

    // Refresh sessions
    await page.click('[data-testid="refresh-btn"]')

    // Check error message
    await expect(page.locator('.error-message')).toContainText('加载会话失败')
  })

  test('should be responsive on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 })

    // Check mobile layout
    await expect(page.locator('[data-testid="mobile-session-card"]')).toBeVisible()
    await expect(page.locator('[data-testid="sessions-table"]')).not.toBeVisible()

    // Check mobile actions
    await page.click('[data-testid="mobile-menu-btn"]')
    await expect(page.locator('[data-testid="mobile-actions-menu"]')).toBeVisible()
  })
})