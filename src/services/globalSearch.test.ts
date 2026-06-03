import { describe, expect, it } from 'vitest'
import { searchGlobalEntries, isStrongSearchMatch, type GlobalSearchEntry } from '@/services/globalSearch'

const entries: GlobalSearchEntry[] = [
  {
    id: 'dashboard',
    title: '仪表盘',
    description: '总览页面',
    path: '/dashboard',
    icon: 'fas fa-home',
    category: 'page',
    keywords: ['dashboard', 'overview', '首页'],
    featured: true,
  },
  {
    id: 'notification-center',
    title: '消息中心',
    description: '查看消息与通知',
    path: '/notification-center',
    icon: 'fas fa-inbox',
    category: 'page',
    keywords: ['message', 'notification', '消息', '通知'],
  },
  {
    id: 'settings',
    title: '系统设置',
    description: '平台配置',
    path: '/settings',
    icon: 'fas fa-cog',
    category: 'page',
    keywords: ['settings', 'config', '配置'],
  },
  {
    id: 'action-theme-dark',
    title: '切换到深色主题',
    description: '立即切换到深色主题',
    path: '',
    icon: 'fas fa-moon',
    category: 'action',
    keywords: ['theme', 'dark'],
    aliases: ['theme dark', 'theme:dark'],
  },
]

describe('globalSearch', () => {
  it('returns ranked matches for a query', () => {
    const results = searchGlobalEntries(entries, '消息')
    expect(results).toHaveLength(1)
    expect(results[0].id).toBe('notification-center')
  })

  it('matches alias keywords', () => {
    const results = searchGlobalEntries(entries, 'overview')
    expect(results[0].id).toBe('dashboard')
  })

  it('prioritizes exact title matches as strong results', () => {
    const [result] = searchGlobalEntries(entries, '系统设置')
    expect(result.id).toBe('settings')
    expect(isStrongSearchMatch(result, '系统设置')).toBe(true)
  })

  it('supports command-style aliases for actions', () => {
    const [result] = searchGlobalEntries(entries, 'theme dark')
    expect(result.id).toBe('action-theme-dark')
    expect(isStrongSearchMatch(result, 'theme dark')).toBe(true)
  })

  it('does not mark partial fuzzy matches as strong results', () => {
    const [result] = searchGlobalEntries(entries, '平台')
    expect(result.id).toBe('settings')
    expect(isStrongSearchMatch(result, '平台')).toBe(false)
  })
})
