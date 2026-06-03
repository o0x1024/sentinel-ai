import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { buildReferencedSurfaceAsset } from '@/utils/referencedSurfaceAsset'
import { buildMentionToken, parseMentionTokens } from './mentionTokenSupport'
import type {
  ReferencedAsset,
  ReferencedConversationMessage,
  ReferencedFile,
  ReferencedTraffic,
} from '@/types/agentReferences'

interface MentionRange {
  start: number
  end: number
  query: string
}

interface WorkingDirectoryFileMatch {
  relative_path: string
  file_name: string
}

interface WorkingDirectoryFilePreview {
  id: string
  path: string
  relative_path: string
  preview: string
  truncated: boolean
  size: number
}

interface SurfaceAssetRow {
  id: string
  asset_type: string
  asset_name: string
  display_name?: string | null
  description?: string | null
  status: string
  risk_level?: string | null
}

interface ProxyRequestMentionMatch {
  id: number
  method: string
  host: string
  url: string
  status_code: number
  timestamp: string
}

interface ProxyRequestCommandResponse<T> {
  success: boolean
  data?: T
  error?: string | null
}

type MentionKind = 'file' | 'asset' | 'message' | 'traffic'
interface MentionableConversationMessage {
  content: string
  id: string
  roleLabel: string
  timestamp: number
  type: string
}

interface MentionSuggestionBase {
  description: string
  id: string
  kind: MentionKind
  label: string
}

export type MentionSuggestionItem =
  | (MentionSuggestionBase & {
    kind: 'file'
    relativePath: string
  })
  | (MentionSuggestionBase & {
    assetId: string
    kind: 'asset'
  })
  | (MentionSuggestionBase & {
    kind: 'message'
    messageId: string
  })
  | (MentionSuggestionBase & {
    kind: 'traffic'
    requestId: number
  })

const MENTION_SEARCH_DEBOUNCE_MS = 120
const MENTION_LIMIT_PER_KIND = 5

const isWhitespace = (char: string | undefined) => !char || /\s/.test(char)

const findMentionRange = (text: string, cursor: number): MentionRange | null => {
  const safeCursor = Math.max(0, Math.min(cursor, text.length))
  let start = safeCursor
  while (start > 0 && !isWhitespace(text[start - 1])) {
    start -= 1
  }

  let end = safeCursor
  while (end < text.length && !isWhitespace(text[end])) {
    end += 1
  }

  const token = text.slice(start, end)
  if (!token.startsWith('@')) return null
  if (start > 0 && !isWhitespace(text[start - 1])) return null

  return {
    start,
    end,
    query: token.slice(1),
  }
}

const removeMentionToken = (text: string, range: MentionRange): { cursor: number; value: string } => {
  const left = text.slice(0, range.start)
  const right = text.slice(range.end)

  if (!left) {
    const trimmedRight = right.replace(/^\s+/, '')
    return { value: trimmedRight, cursor: 0 }
  }

  if (!right) {
    const trimmedLeft = left.replace(/\s+$/, '')
    return { value: trimmedLeft, cursor: trimmedLeft.length }
  }

  if (/\s$/.test(left) || /^\s/.test(right)) {
    return { value: `${left}${right}`, cursor: left.length }
  }

  const value = `${left} ${right}`
  return { value, cursor: left.length + 1 }
}

const replaceMentionToken = (
  text: string,
  range: MentionRange,
  mentionText: string,
): { cursor: number; value: string } => {
  const left = text.slice(0, range.start)
  const right = text.slice(range.end).replace(/^\s*/, '')
  const prefix = left && !/\s$/.test(left) ? `${left} ` : left
  const suffix = right ? ` ${right}` : ''
  const value = `${prefix}${mentionText}${suffix}`
  const cursor = prefix.length + mentionText.length
  return { value, cursor }
}

const getUrlPath = (url: string) => {
  try {
    const parsed = new URL(url)
    return `${parsed.pathname}${parsed.search}`
  } catch {
    const marker = url.indexOf('://')
    const normalized = marker >= 0 ? url.slice(marker + 3) : url
    const slashIndex = normalized.indexOf('/')
    return slashIndex >= 0 ? normalized.slice(slashIndex) : '/'
  }
}

const buildFileItem = (item: WorkingDirectoryFileMatch): MentionSuggestionItem => ({
  description: item.file_name,
  id: `file:${item.relative_path}`,
  kind: 'file',
  label: item.relative_path,
  relativePath: item.relative_path,
})

const buildAssetItem = (item: SurfaceAssetRow): MentionSuggestionItem => ({
  assetId: item.id,
  description: `${item.asset_type} · ${item.risk_level || item.status || 'unknown'}`,
  id: `asset:${item.id}`,
  kind: 'asset',
  label: item.display_name || item.asset_name || item.id,
})

const buildTrafficItem = (item: ProxyRequestMentionMatch): MentionSuggestionItem => ({
  description: `${item.host}${getUrlPath(item.url)} · ${item.status_code || 'N/A'}`,
  id: `traffic:${item.id}`,
  kind: 'traffic',
  label: `${item.method} ${item.host}`,
  requestId: item.id,
})

const buildMessageItem = (item: MentionableConversationMessage): MentionSuggestionItem => ({
  description: item.roleLabel,
  id: `message:${item.id}`,
  kind: 'message',
  label: item.content.replace(/\s+/g, ' ').slice(0, 72) || item.roleLabel,
  messageId: item.id,
})

const buildMentionText = (item: MentionSuggestionItem) => {
  switch (item.kind) {
    case 'file':
      return buildMentionToken({
        id: item.relativePath,
        kind: 'file',
        label: item.relativePath,
      })
    case 'asset':
      return buildMentionToken({
        id: item.assetId,
        kind: 'asset',
        label: item.label,
      })
    case 'message':
      return buildMentionToken({
        id: item.messageId,
        kind: 'message',
        label: item.label,
      })
    case 'traffic':
      return buildMentionToken({
        id: String(item.requestId),
        kind: 'traffic',
        label: item.label,
      })
    default:
      return '@ref'
  }
}

export const useInputMentions = (params: {
  getConversationId: () => string | null
  getInputMessage: () => string
  getReferencedAssets: () => ReferencedAsset[]
  getReferencedFiles: () => ReferencedFile[]
  getReferencedMessages: () => ReferencedConversationMessage[]
  getReferencedTraffic: () => ReferencedTraffic[]
  getConversationMessages: () => MentionableConversationMessage[]
  onAddReferencedAsset: (asset: ReferencedAsset) => void
  onAddReferencedFile: (file: ReferencedFile) => void
  onAddReferencedMessage: (message: ReferencedConversationMessage) => void
  onAddReferencedTraffic: (traffic: ReferencedTraffic) => void
  onInputValueChange: (value: string, cursor?: number) => void
  onSyncReferencedAssets: (assets: ReferencedAsset[]) => void
  onSyncReferencedFiles: (files: ReferencedFile[]) => void
  onSyncReferencedMessages: (messages: ReferencedConversationMessage[]) => void
  onSyncReferencedTraffic: (traffic: ReferencedTraffic[]) => void
}) => {
  const mentionOpen = ref(false)
  const mentionActiveIndex = ref(0)
  const mentionRange = ref<MentionRange | null>(null)
  const mentionLoading = ref(false)
  const mentionError = ref('')
  const mentionItems = ref<MentionSuggestionItem[]>([])
  let searchTimer: ReturnType<typeof setTimeout> | null = null
  let searchSeq = 0

  const filteredMentionItems = computed(() => mentionItems.value)

  const closeMentionPopover = () => {
    mentionOpen.value = false
    mentionActiveIndex.value = 0
    mentionRange.value = null
    mentionError.value = ''
    mentionLoading.value = false
    mentionItems.value = []
    if (searchTimer) {
      clearTimeout(searchTimer)
      searchTimer = null
    }
  }

  const runSearch = async (query: string) => {
    const currentSeq = ++searchSeq
    mentionLoading.value = true
    mentionError.value = ''

    const localMessageItems = params.getConversationMessages()
      .filter((item) => {
        const normalized = query.trim().toLowerCase()
        if (!normalized) return true
        const haystack = `${item.roleLabel} ${item.content}`.toLowerCase()
        return haystack.includes(normalized)
      })
      .slice(0, MENTION_LIMIT_PER_KIND)
      .map(buildMessageItem)

    const [fileResult, assetResult, trafficResult] = await Promise.allSettled([
      invoke<WorkingDirectoryFileMatch[]>('search_working_directory_files', {
        conversationId: params.getConversationId(),
        query,
        limit: MENTION_LIMIT_PER_KIND,
      }),
      invoke<SurfaceAssetRow[]>('surface_list_assets', {
        filter: {
          search: query.trim() || null,
          limit: MENTION_LIMIT_PER_KIND,
        },
      }),
      invoke<ProxyRequestMentionMatch[]>('search_recent_proxy_requests', {
        query,
        limit: MENTION_LIMIT_PER_KIND,
      }),
    ])

    if (currentSeq !== searchSeq) return

    const nextItems: MentionSuggestionItem[] = []
    const errors: string[] = []

    if (fileResult.status === 'fulfilled') {
      nextItems.push(...fileResult.value.map(buildFileItem))
    } else {
      errors.push(fileResult.reason instanceof Error ? fileResult.reason.message : String(fileResult.reason))
    }

    if (assetResult.status === 'fulfilled') {
      nextItems.push(...assetResult.value.map(buildAssetItem))
    } else {
      errors.push(assetResult.reason instanceof Error ? assetResult.reason.message : String(assetResult.reason))
    }

    if (trafficResult.status === 'fulfilled') {
      nextItems.push(...trafficResult.value.map(buildTrafficItem))
    } else {
      errors.push(trafficResult.reason instanceof Error ? trafficResult.reason.message : String(trafficResult.reason))
    }

    nextItems.push(...localMessageItems)

    mentionItems.value = nextItems
    if (mentionActiveIndex.value >= mentionItems.value.length) {
      mentionActiveIndex.value = Math.max(0, mentionItems.value.length - 1)
    }
    mentionError.value = nextItems.length === 0 && errors.length > 0
      ? errors[0]
      : ''
    mentionLoading.value = false
  }

  const scheduleSearch = (query: string) => {
    if (searchTimer) {
      clearTimeout(searchTimer)
    }
    searchTimer = setTimeout(() => {
      void runSearch(query)
    }, MENTION_SEARCH_DEBOUNCE_MS)
  }

  const updateMentionState = (text: string, cursor: number) => {
    const range = findMentionRange(text, cursor)
    if (!range) {
      closeMentionPopover()
      return
    }

    mentionRange.value = range
    mentionOpen.value = true
    mentionError.value = ''
    scheduleSearch(range.query)
  }

  const syncMentionBindings = (text: string) => {
    const tokens = parseMentionTokens(text)
    params.onSyncReferencedFiles(
      params.getReferencedFiles().filter((item) => {
        if (tokens.some((token) => token.kind === 'file' && token.id === item.relativePath)) {
          return true
        }
        return !item.mentionText || text.includes(item.mentionText)
      }),
    )
    params.onSyncReferencedAssets(
      params.getReferencedAssets().filter((item) => {
        if (tokens.some((token) => token.kind === 'asset' && token.id === item.id)) {
          return true
        }
        return !item.mentionText || text.includes(item.mentionText)
      }),
    )
    params.onSyncReferencedMessages(
      params.getReferencedMessages().filter((item) => {
        if (tokens.some((token) => token.kind === 'message' && token.id === item.id)) {
          return true
        }
        return !item.mentionText || text.includes(item.mentionText)
      }),
    )
    params.onSyncReferencedTraffic(
      params.getReferencedTraffic().filter((item) => {
        if (tokens.some((token) => token.kind === 'traffic' && token.id === String(item.id))) {
          return true
        }
        return !item.mentionText || text.includes(item.mentionText)
      }),
    )
  }

  const applyMentionSelection = async (item?: MentionSuggestionItem) => {
    const selectedItem = item || filteredMentionItems.value[Math.max(0, mentionActiveIndex.value)]
    const range = mentionRange.value
    if (!selectedItem || !range) return

    try {
      const mentionText = buildMentionText(selectedItem)

      if (selectedItem.kind === 'file') {
        const preview = await invoke<WorkingDirectoryFilePreview>('read_working_directory_file_preview', {
          conversationId: params.getConversationId(),
          relativePath: selectedItem.relativePath,
          maxChars: 4000,
        })

        params.onAddReferencedFile({
          id: preview.id,
          mentionText,
          path: preview.path,
          relativePath: preview.relative_path,
          preview: preview.preview,
          truncated: preview.truncated,
          size: preview.size,
        })
      } else if (selectedItem.kind === 'asset') {
        const detail = await invoke<any | null>('surface_get_asset_detail', {
          assetId: selectedItem.assetId,
        })
        if (!detail) {
          throw new Error('资产详情不存在')
        }
        params.onAddReferencedAsset({
          ...buildReferencedSurfaceAsset(detail),
          mentionText,
        })
      } else if (selectedItem.kind === 'message') {
        const message = params.getConversationMessages().find((item) => item.id === selectedItem.messageId)
        if (!message) {
          throw new Error('引用消息不存在')
        }
        params.onAddReferencedMessage({
          content: message.content,
          id: message.id,
          mentionText,
          roleLabel: message.roleLabel,
          timestamp: message.timestamp,
          type: message.type,
        })
      } else {
        const response = await invoke<ProxyRequestCommandResponse<ReferencedTraffic | null>>('get_proxy_request', {
          id: selectedItem.requestId,
        })
        if (!response.success || !response.data) {
          throw new Error(response.error || '流量详情不存在')
        }
        params.onAddReferencedTraffic({
          ...response.data,
          mentionText,
        })
      }

      const nextValue = replaceMentionToken(params.getInputMessage(), range, mentionText)
      params.onInputValueChange(nextValue.value, nextValue.cursor)
      closeMentionPopover()
    } catch (error) {
      mentionError.value = error instanceof Error ? error.message : String(error)
    }
  }

  return {
    applyMentionSelection,
    closeMentionPopover,
    filteredMentionItems,
    mentionActiveIndex,
    mentionError,
    mentionLoading,
    mentionOpen,
    syncMentionBindings,
    updateMentionState,
  }
}
