import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Finding } from '@/components/SecurityCenter/vulnerabilityFindingTypes'
import type { WorkbenchCaseListResult } from '@/components/SecurityCenter/securityWorkbenchTypes'

const STORAGE_KEY = 'sentinel-security-center-activity'

const baseFinding: Finding = {
  id: 'finding-1',
  plugin_id: 'plugin.example',
  vuln_type: 'xss',
  severity: 'high',
  confidence: 'high',
  title: 'Reflected XSS',
  description: 'example',
  status: 'open',
  url: 'https://example.com',
  hit_count: 1,
  first_seen_at: '2026-04-08T00:00:00.000Z',
  last_seen_at: '2026-04-08T00:00:00.000Z',
  created_at: '2026-04-08T00:00:00.000Z',
  updated_at: '2026-04-08T00:00:00.000Z',
  evidence: [],
}

const workbenchList: WorkbenchCaseListResult = {
  items: [
    {
      id: 'case-1',
      findingId: 'finding-1',
      title: 'Reflected XSS 调查',
      status: 'new',
      currentConclusion: '',
      priority: 'medium',
      baselineEvidenceId: null,
      createdAt: '2026-04-08T00:00:00.000Z',
      updatedAt: '2026-04-08T00:00:00.000Z',
      lastActivityAt: '2026-04-08T00:00:00.000Z',
      noteCount: 0,
      finding: {
        id: 'finding-1',
        title: 'Reflected XSS',
        vulnType: 'xss',
        severity: 'high',
        confidence: 'high',
        status: 'open',
        pluginId: 'plugin.example',
        url: 'https://example.com',
        description: 'example',
        createdAt: '2026-04-08T00:00:00.000Z',
        updatedAt: '2026-04-08T00:00:00.000Z',
        firstSeenAt: '2026-04-08T00:00:00.000Z',
        lastSeenAt: '2026-04-08T00:00:00.000Z',
        evidence: [],
      },
    },
  ],
  total: 1,
  page: 1,
  pageSize: 20,
}

const mockSecurityCenterResponses = (
  findingItems: Finding[],
  workbenchItems = workbenchList.items,
) => {
  global.testUtils.mockInvoke.mockImplementation((command: string) => {
    if (command === 'list_findings') {
      return Promise.resolve({
        success: true,
        data: findingItems,
      })
    }

    if (command === 'security_workbench_list_cases') {
      return Promise.resolve({
        success: true,
        data: {
          ...workbenchList,
          items: workbenchItems,
          total: workbenchItems.length,
        },
      })
    }

    return Promise.resolve({ success: true, data: null })
  })
}

describe('useSecurityCenterActivity', () => {
  beforeEach(() => {
    vi.resetModules()
    window.localStorage.clear()
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockListen.mockReset().mockResolvedValue(vi.fn())
  })

  it('uses current findings and workbench cases as the initial read baseline', async () => {
    mockSecurityCenterResponses(
      [
        {
          ...baseFinding,
          id: 'finding-newest',
          created_at: '2026-04-10T00:00:00.000Z',
        },
        baseFinding,
      ],
      [
        {
          ...workbenchList.items[0],
          id: 'case-newest',
          lastActivityAt: '2026-04-10T00:00:00.000Z',
        },
      ],
    )

    const { useSecurityCenterActivity } = await import('./useSecurityCenterActivity')
    const activity = useSecurityCenterActivity()

    await activity.initializeSecurityCenterActivity()

    expect(activity.unreadFindingCount.value).toBe(0)
    expect(activity.unreadWorkbenchCaseCount.value).toBe(0)
    expect(activity.unreadSecurityCenterCount.value).toBe(0)
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) || '{}')).toMatchObject({
      baselineSeeded: true,
      readFindingIds: ['finding-newest', 'finding-1'],
      readWorkbenchCaseIds: ['case-newest'],
    })
  })

  it('migrates legacy lastViewedAt and supports marking findings and cases as read', async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ lastViewedAt: '2026-04-08T12:00:00.000Z' }),
    )

    mockSecurityCenterResponses(
      [
        {
          ...baseFinding,
          id: 'finding-new',
          created_at: '2026-04-09T00:00:00.000Z',
        },
        baseFinding,
      ],
      [
        workbenchList.items[0],
        {
          ...workbenchList.items[0],
          id: 'case-new',
          title: '新的工作台案件',
          findingId: 'finding-new',
          lastActivityAt: '2026-04-09T00:00:00.000Z',
          finding: {
            ...workbenchList.items[0].finding,
            id: 'finding-new',
            title: '新的漏洞',
          },
        },
      ],
    )

    const { useSecurityCenterActivity } = await import('./useSecurityCenterActivity')
    const activity = useSecurityCenterActivity()

    await activity.initializeSecurityCenterActivity()
    expect(activity.unreadFindingCount.value).toBe(1)
    expect(activity.unreadWorkbenchCaseCount.value).toBe(1)
    expect(activity.unreadSecurityCenterCount.value).toBe(2)

    activity.markFindingAsRead('finding-new')
    expect(activity.unreadFindingCount.value).toBe(0)
    expect(activity.unreadSecurityCenterCount.value).toBe(1)

    activity.markWorkbenchCaseAsRead('case-new')
    expect(activity.unreadWorkbenchCaseCount.value).toBe(0)
    expect(activity.unreadSecurityCenterCount.value).toBe(0)

    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) || '{}')).toMatchObject({
      baselineSeeded: true,
      legacyMigrated: true,
      readFindingIds: expect.arrayContaining(['finding-1', 'finding-new']),
      readWorkbenchCaseIds: expect.arrayContaining(['case-1', 'case-new']),
    })
  })

  it('does not throw when persisting read state exceeds localStorage quota', async () => {
    mockSecurityCenterResponses([baseFinding], workbenchList.items)

    const setItemSpy = vi.spyOn(window.localStorage.__proto__, 'setItem').mockImplementation(() => {
      throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
    })
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

    const { useSecurityCenterActivity } = await import('./useSecurityCenterActivity')
    const activity = useSecurityCenterActivity()

    expect(() => activity.markFindingAsRead('finding-over-quota')).not.toThrow()
    expect(activity.isFindingRead('finding-over-quota')).toBe(true)

    await activity.initializeSecurityCenterActivity()

    expect(() => activity.markWorkbenchCaseAsRead('case-over-quota')).not.toThrow()
    expect(activity.isWorkbenchCaseRead('case-over-quota')).toBe(true)
    expect(warnSpy).toHaveBeenCalled()

    warnSpy.mockRestore()
    setItemSpy.mockRestore()
  })
})
