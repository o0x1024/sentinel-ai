import { test, expect } from '@playwright/test'

test.describe('Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('should display main dashboard elements', async ({ page }) => {
    // Check page title
    await expect(page).toHaveTitle(/Sentinel AI/)

    // Check main navigation
    await expect(page.locator('[data-testid="navbar"]')).toBeVisible()
    await expect(page.locator('[data-testid="sidebar"]')).toBeVisible()

    // Check stats cards
    await expect(page.locator('[data-testid="stats-grid"]')).toBeVisible()
    await expect(page.locator('.stat')).toHaveCount(4) // Assuming 4 stats cards

    // Check main content sections
    await expect(page.locator('[data-testid="attack-surface"]')).toBeVisible()
    await expect(page.locator('[data-testid="recent-vulnerabilities"]')).toBeVisible()

  })

  test('should navigate between pages', async ({ page }) => {
    // Test navigation to Scan Tasks
    await page.click('[data-testid="nav-scan-tasks"]')
    await expect(page.locator('h1')).toContainText('扫描任务')
    await expect(page.url()).toContain('/scan-tasks')

    // Test navigation to Vulnerabilities
    await page.click('[data-testid="nav-vulnerabilities"]')
    await expect(page.locator('h1')).toContainText('漏洞管理')
    await expect(page.url()).toContain('/vulnerabilities')

    // Return to Dashboard
    await page.click('[data-testid="nav-dashboard"]')
    await expect(page.url()).toBe('http://127.0.0.1:1499/')
  })

  test('should display and interact with stats cards', async ({ page }) => {
    const statsCards = page.locator('.stat')
    
    // Check that all cards are visible
    await expect(statsCards).toHaveCount(4)

    // Check card content

    await expect(statsCards.nth(1)).toContainText('Active Scans')
    await expect(statsCards.nth(2)).toContainText('Vulnerabilities')
    await expect(statsCards.nth(3)).toContainText('Success Rate')

    // Test clicking on a stats card
    await statsCards.first().click()
    // Should navigate or show details (depending on implementation)
  })

  test('should display attack surface information', async ({ page }) => {
    const attackSurface = page.locator('[data-testid="attack-surface"]')
    
    await expect(attackSurface).toBeVisible()
    await expect(attackSurface.locator('h2')).toContainText('攻击面')

    // Check tabs
    await expect(attackSurface.locator('[role="tab"]')).toHaveCount(3)
    
    // Test tab switching
    await attackSurface.locator('[role="tab"]').nth(1).click()
    await expect(attackSurface.locator('[role="tabpanel"]')).toBeVisible()
  })

  test('should display recent vulnerabilities', async ({ page }) => {
    const vulnerabilities = page.locator('[data-testid="recent-vulnerabilities"]')
    
    await expect(vulnerabilities).toBeVisible()
    await expect(vulnerabilities.locator('h2')).toContainText('最新漏洞')

    // Check if vulnerabilities are displayed
    const vulnItems = vulnerabilities.locator('.vulnerability-item')
    if (await vulnItems.count() > 0) {
      await expect(vulnItems.first()).toBeVisible()
      
      // Test clicking on vulnerability item
      await vulnItems.first().click()
      // Should show vulnerability details or navigate
    }
  })



  test('should show floating chat interface', async ({ page }) => {
    const floatingChat = page.locator('[data-testid="floating-chat"]')
    
    await expect(floatingChat).toBeVisible()

    // Test opening chat
    await floatingChat.click()
    await expect(page.locator('[data-testid="chat-window"]')).toBeVisible()

    // Test typing in chat
    await page.fill('[data-testid="chat-input"]', 'Hello AI assistant')
    await page.press('[data-testid="chat-input"]', 'Enter')

    // Should see message in chat
    await expect(page.locator('[data-testid="chat-messages"]')).toContainText('Hello AI assistant')
  })

  test('should handle theme switching', async ({ page }) => {
    const themeSwitch = page.locator('[data-testid="theme-switch"]')
    
    if (await themeSwitch.isVisible()) {
      await themeSwitch.click()
      
      // Check if theme changed (HTML attribute or class)
      const html = page.locator('html')
      await expect(html).toHaveAttribute('data-theme', /dark|light/)
    }
  })

  test('should be responsive on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 })

    // Check mobile navigation
    const mobileNav = page.locator('[data-testid="mobile-nav-toggle"]')
    if (await mobileNav.isVisible()) {
      await mobileNav.click()
      await expect(page.locator('[data-testid="mobile-nav-menu"]')).toBeVisible()
    }

    // Check that content is still accessible
    await expect(page.locator('[data-testid="stats-grid"]')).toBeVisible()
  })

  test('should handle error states gracefully', async ({ page }) => {
    // Mock network failure
    await page.route('**/api/**', route => route.abort())

    // Reload page to trigger error state
    await page.reload()

    // Should show error message or fallback UI
    const errorMessage = page.locator('[data-testid="error-message"]')
    if (await errorMessage.isVisible()) {
      await expect(errorMessage).toContainText('error', { ignoreCase: true })
    }
  })
})