import { computed } from 'vue'
import type { Router } from 'vue-router'
import type { GlobalSearchEntry } from '@/services/globalSearch'

function applyTheme(theme: string) {
  document.documentElement.setAttribute('data-theme', theme)
  localStorage.setItem('theme', theme)
}

export function useCommandPaletteActions(router: Router) {
  const actions = computed<GlobalSearchEntry[]>(() => [
    {
      id: 'action:theme-light',
      title: '切换到浅色主题',
      description: '立即切换应用主题为浅色',
      path: '',
      icon: 'fas fa-sun',
      category: 'action',
      keywords: ['theme', 'light', '浅色', '主题'],
      aliases: ['theme light', 'theme:light', 'light theme', '切换浅色主题'],
      featured: true,
      execute: () => applyTheme('light'),
    },
    {
      id: 'action:theme-dark',
      title: '切换到深色主题',
      description: '立即切换应用主题为深色',
      path: '',
      icon: 'fas fa-moon',
      category: 'action',
      keywords: ['theme', 'dark', '深色', '主题'],
      aliases: ['theme dark', 'theme:dark', 'dark theme', '切换深色主题'],
      featured: true,
      execute: () => applyTheme('dark'),
    },
    {
      id: 'action:theme-corporate',
      title: '切换到企业主题',
      description: '立即切换应用主题为企业风格',
      path: '',
      icon: 'fas fa-building',
      category: 'action',
      keywords: ['theme', 'corporate', '企业', '主题'],
      aliases: ['theme corporate', 'theme:corporate', 'corporate theme', '企业主题'],
      execute: () => applyTheme('corporate'),
    },
    {
      id: 'action:notifications-rules',
      title: '打开通知规则',
      description: '前往通知规则与偏好设置页面',
      path: '/notifications',
      icon: 'fas fa-sliders',
      category: 'action',
      keywords: ['notification rules', '通知规则', '通知', '规则'],
      aliases: ['notify rules', 'notification rules', 'rule notifications', '通知规则'],
      featured: true,
      execute: () => router.push('/notifications'),
    },
    {
      id: 'action:notification-center',
      title: '打开消息中心',
      description: '查看所有系统消息与通知',
      path: '/notification-center',
      icon: 'fas fa-inbox',
      category: 'action',
      keywords: ['message center', '消息中心', '通知', '消息'],
      aliases: ['notify center', 'notification center', 'message center', '消息中心'],
      execute: () => router.push('/notification-center'),
    },
    {
      id: 'action:notification-messages',
      title: '打开消息中心 / 消息',
      description: '只查看消息分类',
      path: '/notification-center',
      icon: 'fas fa-envelope',
      category: 'action',
      keywords: ['message', 'messages', '消息', '站内信'],
      aliases: ['notify open messages', 'notify messages', 'message inbox', '打开消息'],
      execute: () => router.push({
        path: '/notification-center',
        query: {
          category: 'message',
        },
      }),
    },
    {
      id: 'action:notification-notifications',
      title: '打开消息中心 / 通知',
      description: '只查看通知分类',
      path: '/notification-center',
      icon: 'fas fa-bell',
      category: 'action',
      keywords: ['notification', 'notifications', '通知', '提醒'],
      aliases: ['notify open notifications', 'notify notifications', 'open notifications', '打开通知'],
      execute: () => router.push({
        path: '/notification-center',
        query: {
          category: 'notification',
        },
      }),
    },
    {
      id: 'action:critical-findings',
      title: '查看严重漏洞',
      description: '打开漏洞列表并筛选严重级别',
      path: '/security-center',
      icon: 'fas fa-radiation',
      category: 'action',
      keywords: ['critical', '漏洞', '严重', 'finding'],
      aliases: ['finding critical', 'findings critical', 'critical findings', '严重漏洞'],
      featured: true,
      execute: () =>
        router.push({
          path: '/security-center',
          query: {
            tab: 'vulnerabilities',
            severity: 'critical',
          },
        }),
    },
    {
      id: 'action:high-findings',
      title: '查看高危漏洞',
      description: '打开漏洞列表并筛选高危级别',
      path: '/security-center',
      icon: 'fas fa-triangle-exclamation',
      category: 'action',
      keywords: ['high', '漏洞', '高危', 'finding'],
      aliases: ['finding high', 'findings high', 'high findings', '高危漏洞'],
      featured: true,
      execute: () =>
        router.push({
          path: '/security-center',
          query: {
            tab: 'vulnerabilities',
            severity: 'high',
          },
        }),
    },
    {
      id: 'action:medium-findings',
      title: '查看中危漏洞',
      description: '打开漏洞列表并筛选中危级别',
      path: '/security-center',
      icon: 'fas fa-circle-exclamation',
      category: 'action',
      keywords: ['medium', '漏洞', '中危', 'finding'],
      aliases: ['finding medium', 'findings medium', 'medium findings', '中危漏洞'],
      execute: () =>
        router.push({
          path: '/security-center',
          query: {
            tab: 'vulnerabilities',
            severity: 'medium',
          },
        }),
    },
    {
      id: 'action:low-findings',
      title: '查看低危漏洞',
      description: '打开漏洞列表并筛选低危级别',
      path: '/security-center',
      icon: 'fas fa-shield-virus',
      category: 'action',
      keywords: ['low', '漏洞', '低危', 'finding'],
      aliases: ['finding low', 'findings low', 'low findings', '低危漏洞'],
      execute: () =>
        router.push({
          path: '/security-center',
          query: {
            tab: 'vulnerabilities',
            severity: 'low',
          },
        }),
    },
  ])

  return {
    actions,
  }
}
