import type { GlobalSearchEntry } from '@/services/globalSearch'

export interface SearchScanTaskItem {
  id: string
  name?: string
  target?: string
  status?: string
  created_at?: string
  createdAt?: string
}

export interface SearchWorkflowDefinitionItem {
  id: string
  name?: string
  description?: string
  tags?: string[] | string
  version?: string
  is_tool?: boolean
  isTool?: boolean
}

export interface SearchWorkflowRunItem {
  id: string
  workflow_id?: string
  workflowId?: string
  workflow_name?: string
  workflowName?: string
  status?: string
  summary?: string
  started_at?: string
  startedAt?: string
}

export interface SearchKnowledgeDocumentItem {
  id: string
  file_name?: string
  fileName?: string
  collection_id?: string
  collectionId?: string
  collection_name?: string
  collectionName?: string
  source_type?: string
  sourceType?: string
  chunk_count?: number
  chunkCount?: number
  summary?: string
}

export interface SearchBountyKnowledgeItem {
  id: string
  title?: string
  content?: string
  snippet?: string
  summary?: string
  program_id?: string | null
  programId?: string | null
  program_name?: string | null
  programName?: string | null
  tags_json?: string | null
  updated_at?: string
  updatedAt?: string
}

export interface SearchPluginItem {
  plugin_id: string
  plugin_name?: string
  name?: string
  title?: string
  description?: string
  author?: string
  status?: string
  language?: string
}

function normalizeKeywords(values: Array<string | number | undefined | null>) {
  return values
    .map(value => String(value || '').trim())
    .filter(Boolean)
}

function normalizeWorkflowTags(tags?: string[] | string) {
  if (Array.isArray(tags)) {
    return normalizeKeywords(tags)
  }

  if (typeof tags === 'string') {
    return normalizeKeywords(tags.split(/[,\s]+/g))
  }

  return []
}

export function createScanTaskSearchEntries(items: SearchScanTaskItem[]): GlobalSearchEntry[] {
  return items
    .slice(0, 20)
    .filter(item => item?.id)
    .map(item => ({
      id: `scan-task:${item.id}`,
      title: item.name?.trim() || `扫描任务 ${item.id}`,
      description: item.target?.trim()
        ? `目标 ${item.target} · 状态 ${item.status || 'unknown'}`
        : `状态 ${item.status || 'unknown'}`,
      path: '/scan-tasks',
      query: {
        taskId: item.id,
      },
      icon: 'fas fa-list-check',
      category: 'task',
      keywords: normalizeKeywords([
        item.id,
        item.name,
        item.target,
        item.status,
        item.created_at,
        item.createdAt,
        'scan task',
        '扫描任务',
      ]),
    }))
}

export function createWorkflowDefinitionSearchEntries(items: SearchWorkflowDefinitionItem[]): GlobalSearchEntry[] {
  return items
    .slice(0, 20)
    .filter(item => item?.id)
    .map(item => ({
      id: `workflow-definition:${item.id}`,
      title: item.name?.trim() || `工作流 ${item.id}`,
      description: item.description?.trim() || (item.is_tool || item.isTool ? '工作流工具定义' : '工作流定义'),
      path: '/workflow-studio',
      query: {
        workflowId: item.id,
      },
      icon: item.is_tool || item.isTool ? 'fas fa-screwdriver-wrench' : 'fas fa-project-diagram',
      category: 'workflow',
      keywords: normalizeKeywords([
        item.id,
        item.name,
        item.description,
        item.version,
        item.is_tool || item.isTool ? 'tool' : 'workflow',
        ...(normalizeWorkflowTags(item.tags)),
      ]),
    }))
}

export function createWorkflowRunSearchEntries(items: SearchWorkflowRunItem[]): GlobalSearchEntry[] {
  return items
    .slice(0, 20)
    .filter(item => item?.id)
    .map(item => ({
      id: `workflow-run:${item.id}`,
      title: item.workflow_name?.trim() || item.workflowName?.trim() || `工作流运行 ${item.id}`,
      description: item.summary?.trim()
        || `运行状态 ${item.status || 'unknown'} · 执行记录 ${item.id}`,
      path: '/workflow-studio',
      query: {
        execution_id: item.id,
      },
      icon: 'fas fa-play-circle',
      category: 'workflow',
      keywords: normalizeKeywords([
        item.id,
        item.workflow_id,
        item.workflowId,
        item.workflow_name,
        item.workflowName,
        item.status,
        item.started_at,
        item.startedAt,
        'workflow run',
        'execution',
        '工作流运行',
      ]),
    }))
}

export function createKnowledgeDocumentSearchEntries(items: SearchKnowledgeDocumentItem[]): GlobalSearchEntry[] {
  return items
    .slice(0, 24)
    .filter(item => item?.id)
    .map(item => {
      const title = item.file_name?.trim() || item.fileName?.trim() || `文档 ${item.id}`
      const collectionName = item.collection_name?.trim() || item.collectionName?.trim() || '默认集合'
      const chunkCount = item.chunk_count ?? item.chunkCount

      return {
        id: `knowledge-document:${item.id}`,
        title,
        description: item.summary?.trim() || `${collectionName}${chunkCount ? ` · ${chunkCount} 个分块` : ''}`,
        path: '/rag-management',
        query: {
          collectionId: item.collection_id || item.collectionId || '',
          documentId: item.id,
        },
        icon: 'fas fa-file-lines',
        category: 'document',
        keywords: normalizeKeywords([
          item.id,
          title,
          collectionName,
          item.source_type,
          item.sourceType,
          chunkCount,
          'knowledge',
          'document',
          'rag',
          '知识库',
          '文档',
        ]),
      }
    })
}

function normalizeJsonTags(tags?: string | null) {
  if (!tags) {
    return []
  }

  try {
    const parsed = JSON.parse(tags)
    return Array.isArray(parsed) ? normalizeKeywords(parsed) : []
  } catch {
    return normalizeKeywords(tags.split(/[,\s]+/g))
  }
}

function stripSnippetMarkup(value?: string) {
  return String(value || '').replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim()
}

export function createBountyKnowledgeSearchEntries(items: SearchBountyKnowledgeItem[]): GlobalSearchEntry[] {
  return items
    .slice(0, 24)
    .filter(item => item?.id)
    .map(item => {
      const title = item.title?.trim() || `赏金笔记 ${item.id}`
      const programName = item.program_name?.trim() || item.programName?.trim() || '未绑定项目'
      const description = stripSnippetMarkup(item.snippet) || item.summary?.trim() || item.content?.trim() || programName

      return {
        id: `bounty-knowledge:${item.id}`,
        title,
        description,
        path: '/bug-bounty',
        query: {
          tab: 'knowledge',
          knowledgeId: item.id,
        },
        icon: 'fas fa-note-sticky',
        category: 'document',
        keywords: normalizeKeywords([
          item.id,
          title,
          description,
          item.program_id,
          item.programId,
          programName,
          item.updated_at,
          item.updatedAt,
          ...normalizeJsonTags(item.tags_json),
          'bug bounty',
          'knowledge',
          'notes',
          '漏洞赏金',
          '知识库',
          '笔记',
        ]),
      }
    })
}

export function createPluginSearchEntries(items: SearchPluginItem[]): GlobalSearchEntry[] {
  return items
    .slice(0, 24)
    .filter(item => item?.plugin_id)
    .map(item => {
      const title = item.title?.trim() || item.plugin_name?.trim() || item.name?.trim() || item.plugin_id
      return {
        id: `plugin:${item.plugin_id}`,
        title,
        description: item.description?.trim() || `状态 ${item.status || 'unknown'}${item.author ? ` · 作者 ${item.author}` : ''}`,
        path: '/plugins',
        query: {
          pluginId: item.plugin_id,
        },
        icon: 'fas fa-puzzle-piece',
        category: 'plugin',
        keywords: normalizeKeywords([
          item.plugin_id,
          title,
          item.author,
          item.status,
          item.language,
          'plugin',
          '插件',
        ]),
      }
    })
}
