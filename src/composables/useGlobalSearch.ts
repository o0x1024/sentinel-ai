import { computed, unref, type ComputedRef, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Finding } from '@/components/SecurityCenter/vulnerabilityFindingTypes'
import type { GlobalSearchEntry } from '@/services/globalSearch'
import { useFeatureEntitlementsState } from '@/services/featureEntitlements'
import { searchGlobalEntries } from '@/services/globalSearch'
import {
  createBountyKnowledgeSearchEntries,
  createKnowledgeDocumentSearchEntries,
  createPluginSearchEntries,
  createScanTaskSearchEntries,
  createWorkflowDefinitionSearchEntries,
  createWorkflowRunSearchEntries,
  type SearchBountyKnowledgeItem,
  type SearchKnowledgeDocumentItem,
  type SearchPluginItem,
  type SearchScanTaskItem,
  type SearchWorkflowDefinitionItem,
  type SearchWorkflowRunItem,
} from '@/services/searchEntryBuilders'
import type { AppNotificationItem } from '@/types/notification'

type TranslateFn = (key: string, fallback: string) => string
type SearchEntrySource<T> = T | Ref<T> | ComputedRef<T>

export function createGlobalSearchEntries(t: TranslateFn): GlobalSearchEntry[] {
  return [
    {
      id: 'dashboard',
      title: t('sidebar.dashboard', '仪表盘'),
      description: t('dashboard.welcome', '欢迎回来'),
      path: '/dashboard',
      icon: 'fas fa-home',
      category: 'page',
      keywords: ['dashboard', 'home', 'overview', '总览', '首页'],
      featured: true,
    },
    {
      id: 'security-center',
      title: t('sidebar.securityCenter', '安全中心'),
      description: '漏洞、扫描任务与风险视图',
      path: '/security-center',
      icon: 'fas fa-shield-alt',
      category: 'page',
      keywords: ['security', 'vulnerability', 'scan', '漏洞', '扫描任务', '风险'],
      featured: true,
    },
    {
      id: 'scan-tasks',
      title: '扫描任务',
      description: '查看任务运行状态、进度与历史',
      path: '/scan-tasks',
      icon: 'fas fa-list-check',
      category: 'shortcut',
      keywords: ['task', 'tasks', 'scan tasks', '任务', '扫描'],
    },
    {
      id: 'vulnerabilities',
      title: '漏洞管理',
      description: '查看漏洞列表、状态与处置建议',
      path: '/vulnerabilities',
      icon: 'fas fa-bug',
      category: 'shortcut',
      keywords: ['vulnerability', 'weakness', 'cve', '漏洞', '风险项'],
    },
    {
      id: 'traffic',
      title: t('sidebar.traffic', '流量分析'),
      description: '代理、拦截、重放和流量历史',
      path: '/traffic',
      icon: 'fas fa-satellite-dish',
      category: 'page',
      keywords: ['traffic', 'proxy', 'history', 'repeater', '流量', '代理', '重放'],
      featured: true,
    },
    {
      id: 'ai-assistant',
      title: t('sidebar.aiAssistant', 'AI助手'),
      description: '与智能体对话、使用工具与工作区',
      path: '/ai-assistant',
      icon: 'fas fa-brain',
      category: 'page',
      keywords: ['ai', 'assistant', 'agent', 'chat', '智能体', '对话'],
      featured: true,
    },
    {
      id: 'workflow-studio',
      title: t('sidebar.workflowStudio', '工作流'),
      description: '编排、执行和回放工作流',
      path: '/workflow-studio',
      icon: 'fas fa-project-diagram',
      category: 'page',
      keywords: ['workflow', 'flow', 'studio', '编排', '工作流'],
      featured: true,
    },
    {
      id: 'bug-bounty',
      title: t('sidebar.bugBounty', '漏洞赏金'),
      description: '赏金项目、资产与目标管理',
      path: '/bug-bounty',
      icon: 'fas fa-trophy',
      category: 'page',
      keywords: ['bug bounty', 'bounty', 'program', '赏金', '漏洞赏金'],
    },
    {
      id: 'mcp-tools',
      title: t('sidebar.Tools', '应用工具'),
      description: '内置工具、MCP 服务和技能',
      path: '/mcp-tools',
      icon: 'fas fa-tools',
      category: 'page',
      keywords: ['tool', 'tools', 'mcp', 'skills', '工具', '技能'],
      featured: true,
    },
    {
      id: 'cyberchef',
      title: t('sidebar.cyberChef', 'CyberChef'),
      description: '编码、解码、转换与数据处理',
      path: '/cyberchef',
      icon: 'fas fa-terminal',
      category: 'page',
      keywords: ['cyberchef', 'encode', 'decode', 'crypto', '编码', '解码'],
    },
    {
      id: 'dictionary',
      title: t('sidebar.dictionary', '字典管理'),
      description: '字典、词库和 payload 管理',
      path: '/dictionary',
      icon: 'fas fa-book',
      category: 'page',
      keywords: ['dictionary', 'wordlist', 'payload', '字典', '词库'],
    },
    {
      id: 'plugins',
      title: t('sidebar.plugins', '插件管理'),
      description: '插件审查、编辑和发布',
      path: '/plugins',
      icon: 'fas fa-puzzle-piece',
      category: 'page',
      keywords: ['plugin', 'plugins', 'extension', '插件'],
    },
    {
      id: 'rag-management',
      title: t('sidebar.ragManagement', '知识库管理'),
      description: '集合、文档与检索配置',
      path: '/rag-management',
      icon: 'fas fa-database',
      category: 'page',
      keywords: ['rag', 'knowledge base', 'document', '知识库', '文档'],
    },
    {
      id: 'notification-center',
      title: t('sidebar.notificationCenter', '消息中心'),
      description: '查看系统消息、提醒与动态',
      path: '/notification-center',
      icon: 'fas fa-inbox',
      category: 'page',
      keywords: ['notification', 'message', 'inbox', '通知', '消息'],
    },
    {
      id: 'notification-center-messages',
      title: '消息中心 / 消息',
      description: '直接打开消息分类',
      path: '/notification-center',
      query: { category: 'message' },
      icon: 'fas fa-envelope',
      category: 'shortcut',
      keywords: ['messages', 'message', 'inbox', '站内信', '消息'],
    },
    {
      id: 'notification-center-notifications',
      title: '消息中心 / 通知',
      description: '直接打开通知分类',
      path: '/notification-center',
      query: { category: 'notification' },
      icon: 'fas fa-bell',
      category: 'shortcut',
      keywords: ['notifications', 'notification', 'alert', '通知', '提醒'],
    },
    {
      id: 'agent-management',
      title: t('sidebar.agentManagement', '智能体管理'),
      description: '查看和管理被动系统智能体',
      path: '/agent-management',
      icon: 'fas fa-robot',
      category: 'page',
      keywords: ['agent', 'agents', 'robot', '智能体', 'system agent', '被动智能体'],
    },
    {
      id: 'notifications',
      title: t('sidebar.notificationRules', '通知中心'),
      description: '通知规则、偏好与策略设置',
      path: '/notifications',
      icon: 'fas fa-sliders',
      category: 'page',
      keywords: ['rule', 'rules', 'notification rules', '通知规则', '策略'],
    },
    {
      id: 'performance',
      title: t('sidebar.performance', '性能监控'),
      description: '查看应用性能与运行指标',
      path: '/performance',
      icon: 'fas fa-chart-line',
      category: 'page',
      keywords: ['performance', 'metrics', 'monitor', '性能', '监控'],
    },
    {
      id: 'settings',
      title: t('sidebar.settings', '系统设置'),
      description: '系统、模型和平台配置',
      path: '/settings',
      icon: 'fas fa-cog',
      category: 'page',
      keywords: ['settings', 'config', 'preferences', '系统设置', '配置'],
      featured: true,
    },
  ]
}

function mapNotificationSourceLabel(source: AppNotificationItem['source']) {
  switch (source) {
    case 'ai_assistant':
      return 'AI 助手'
    case 'workflow':
      return '工作流'
    case 'monitor':
      return '监控'
    case 'bug_bounty_workflow':
      return '漏洞赏金'
    default:
      return source
  }
}

export function createNotificationSearchEntries(items: AppNotificationItem[]): GlobalSearchEntry[] {
  return items.slice(0, 30).map(item => {
    const categoryLabel = item.category === 'message' ? '消息' : '通知'
    const sourceLabel = mapNotificationSourceLabel(item.source)

    return {
      id: `notification:${item.id}`,
      title: item.title,
      description: item.message,
      path: item.route?.path || '/notification-center',
      query: item.route?.query || { category: item.category },
      icon: item.icon,
      category: 'notification',
      keywords: [
        categoryLabel,
        sourceLabel,
        item.source,
        item.category,
        item.level,
        item.read ? '已读' : '未读',
      ],
    }
  })
}

export function createFindingSearchEntries(items: Finding[]): GlobalSearchEntry[] {
  return items.slice(0, 30).map(item => ({
    id: `finding:${item.id}`,
    title: item.title,
    description: item.description || item.url || item.vuln_type,
    path: '/security-center',
    query: {
      tab: 'vulnerabilities',
      findingId: item.id,
    },
    icon: 'fas fa-bug',
    category: 'finding',
    keywords: [
      item.severity || '',
      item.status || '',
      item.confidence || '',
      item.vuln_type || '',
      item.url || '',
      item.cwe || '',
      item.owasp || '',
      item.plugin_id || '',
    ].filter(Boolean),
  }))
}

export function useGlobalSearch(options?: {
  notifications?: SearchEntrySource<AppNotificationItem[]>
  findings?: SearchEntrySource<Finding[]>
  scanTasks?: SearchEntrySource<SearchScanTaskItem[]>
  workflowDefinitions?: SearchEntrySource<SearchWorkflowDefinitionItem[]>
  workflowRuns?: SearchEntrySource<SearchWorkflowRunItem[]>
  knowledgeDocuments?: SearchEntrySource<SearchKnowledgeDocumentItem[]>
  bountyKnowledgeNotes?: SearchEntrySource<SearchBountyKnowledgeItem[]>
  plugins?: SearchEntrySource<SearchPluginItem[]>
}) {
  const { t } = useI18n()
  const entitlements = useFeatureEntitlementsState()
  const entries = computed(() => {
    const staticEntries = createGlobalSearchEntries((key, fallback) => t(key, fallback))
      .filter(entry => {
        if (entry.path === '/bug-bounty' && !entitlements.value.can_access_bug_bounty) {
          return false
        }
        return true
      })
    const notifications = createNotificationSearchEntries(unref(options?.notifications) || [])
    const findings = createFindingSearchEntries(unref(options?.findings) || [])
    const scanTasks = createScanTaskSearchEntries(unref(options?.scanTasks) || [])
    const workflowDefinitions = createWorkflowDefinitionSearchEntries(unref(options?.workflowDefinitions) || [])
    const workflowRuns = createWorkflowRunSearchEntries(unref(options?.workflowRuns) || [])
    const knowledgeDocuments = createKnowledgeDocumentSearchEntries(unref(options?.knowledgeDocuments) || [])
    const bountyKnowledgeNotes = createBountyKnowledgeSearchEntries(unref(options?.bountyKnowledgeNotes) || [])
    const plugins = createPluginSearchEntries(unref(options?.plugins) || [])
    return [
      ...staticEntries,
      ...notifications,
      ...findings,
      ...scanTasks,
      ...workflowDefinitions,
      ...workflowRuns,
      ...knowledgeDocuments,
      ...bountyKnowledgeNotes,
      ...plugins,
    ]
  })

  return {
    entries,
    search: (query: string, limit?: number) => searchGlobalEntries(entries.value, query, limit),
  }
}
