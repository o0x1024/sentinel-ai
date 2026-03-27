import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'

type SlashCommandType = 'prompt' | 'action'
type SlashCommandScope = 'global' | 'conversation'
type SlashActionType =
  | 'new_conversation'
  | 'clear_conversation'
  | 'toggle_rag'
  | 'toggle_tools'
  | 'open_tool_config'

interface SlashCommandItem {
  id: string
  name: string
  description?: string
  type: SlashCommandType
  template?: string
  action?: SlashActionType
  auto_send?: boolean
  enabled: boolean
  scope?: SlashCommandScope
  sort?: number
  is_builtin?: boolean
}

const SLASH_COMMANDS_CATEGORY = 'agent'
const SLASH_COMMANDS_KEY = 'slash_commands'

export const useInputSlashCommands = (params: {
  conversationId: ComputedRef<string | null | undefined>
  createNewConversation: () => void
  clearConversation: () => void
  emitOpenToolConfig: () => void
  emitSend: () => void
  getInputMessage: () => string
  onInputValueChange: (value: string) => void
  toggleRAG: () => void
  toggleTools: () => void
}) => {
  const slashOpen = ref(false)
  const slashActiveIndex = ref(0)
  const slashQuery = ref('')
  const slashRange = ref<{ start: number; end: number } | null>(null)
  const showSlashManager = ref(false)
  const editingSlashId = ref<string | null>(null)
  const slashFormError = ref('')
  const slashManagerScope = ref<SlashCommandScope>('global')
  const globalSlashCommands = ref<SlashCommandItem[]>([])
  const conversationSlashCommands = ref<Record<string, SlashCommandItem[]>>({})
  const importSlashInputRef = ref<HTMLInputElement | null>(null)
  const conversationScopeEnabled = computed(() => !!params.conversationId.value)
  const conversationScopeKey = computed(() => params.conversationId.value || '__no_conversation__')

  const builtinSlashCommands = computed<SlashCommandItem[]>(() => [
    {
      id: 'builtin-new',
      name: 'new',
      description: '新建会话',
      type: 'action',
      action: 'new_conversation',
      enabled: true,
      is_builtin: true,
    },
    {
      id: 'builtin-clear',
      name: 'clear',
      description: '清空当前会话',
      type: 'action',
      action: 'clear_conversation',
      enabled: true,
      is_builtin: true,
    },
    {
      id: 'builtin-rag',
      name: 'rag',
      description: '切换 RAG',
      type: 'action',
      action: 'toggle_rag',
      enabled: true,
      is_builtin: true,
    },
    {
      id: 'builtin-tools',
      name: 'tools',
      description: '切换 Tools',
      type: 'action',
      action: 'toggle_tools',
      enabled: true,
      is_builtin: true,
    },
    {
      id: 'builtin-toolcfg',
      name: 'toolcfg',
      description: '打开工具配置',
      type: 'action',
      action: 'open_tool_config',
      enabled: true,
      is_builtin: true,
    },
  ])

  const sortByOrder = (a: SlashCommandItem, b: SlashCommandItem) => (a.sort ?? 0) - (b.sort ?? 0)

  const currentConversationCommands = computed(() => {
    const key = conversationScopeKey.value
    return (conversationSlashCommands.value[key] || []).slice().sort(sortByOrder)
  })

  const runtimeCustomCommands = computed(() => {
    const map = new Map<string, SlashCommandItem>()
    globalSlashCommands.value
      .filter((command) => command.enabled !== false)
      .slice()
      .sort(sortByOrder)
      .forEach((command) => map.set(command.name.toLowerCase(), command))
    currentConversationCommands.value
      .filter((command) => command.enabled !== false)
      .forEach((command) => map.set(command.name.toLowerCase(), command))
    return [...map.values()]
  })

  const slashCommands = computed(() => [...builtinSlashCommands.value, ...runtimeCustomCommands.value])

  const filteredSlashCommands = computed(() => {
    const query = slashQuery.value.trim().toLowerCase()
    const list = slashCommands.value.filter((command) => {
      if (!query) return true
      const text = `${command.name} ${command.description || ''}`.toLowerCase()
      return text.includes(query)
    })
    return list.slice(0, 20)
  })

  const scopeCommandsForManager = computed(() => {
    if (slashManagerScope.value === 'conversation') {
      return currentConversationCommands.value
    }
    return globalSlashCommands.value.slice().sort(sortByOrder)
  })

  const managerBuiltinCommands = computed(() => {
    return slashManagerScope.value === 'global' ? builtinSlashCommands.value : []
  })

  const managerCustomCommands = computed<SlashCommandItem[]>({
    get: () => scopeCommandsForManager.value,
    set: (value) => {
      if (slashManagerScope.value === 'conversation') {
        const key = conversationScopeKey.value
        conversationSlashCommands.value = {
          ...conversationSlashCommands.value,
          [key]: value,
        }
      } else {
        globalSlashCommands.value = value
      }
    },
  })

  const newSlashCommand = ref<SlashCommandItem>({
    id: '',
    name: '',
    description: '',
    type: 'prompt',
    template: '{{input}}',
    action: 'new_conversation',
    auto_send: false,
    enabled: true,
    scope: 'global',
    sort: 0,
  })

  const getActionLabel = (action?: SlashActionType) => {
    switch (action) {
      case 'new_conversation':
        return '新建会话'
      case 'clear_conversation':
        return '清空会话'
      case 'toggle_rag':
        return '切换 RAG'
      case 'toggle_tools':
        return '切换 Tools'
      case 'open_tool_config':
        return '打开工具配置'
      default:
        return ''
    }
  }

  const getScopeCommandsMutable = (scope: SlashCommandScope): SlashCommandItem[] => {
    if (scope === 'conversation') {
      const key = conversationScopeKey.value
      if (!conversationSlashCommands.value[key]) {
        conversationSlashCommands.value[key] = []
      }
      return conversationSlashCommands.value[key]
    }
    return globalSlashCommands.value
  }

  const getScopeLabel = (scope?: SlashCommandScope) => (scope === 'conversation' ? '会话' : '全局')

  const generateCommandId = (): string => {
    try {
      if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
        return crypto.randomUUID()
      }
    } catch {
      // ignore and fallback
    }
    return `slash_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
  }

  const normalizeSlashCommand = (
    item: Partial<SlashCommandItem>,
    fallbackScope: SlashCommandScope,
    fallbackSort: number,
  ): SlashCommandItem => {
    const scope: SlashCommandScope = item.scope === 'conversation' ? 'conversation' : fallbackScope
    return {
      id: item.id || generateCommandId(),
      name: String(item.name || '').replace(/^\//, ''),
      description: typeof item.description === 'string' ? item.description : '',
      type: item.type === 'action' ? 'action' : 'prompt',
      template: typeof item.template === 'string' ? item.template : '',
      action: item.action,
      auto_send: !!item.auto_send,
      enabled: item.enabled !== false,
      scope,
      sort: typeof item.sort === 'number' ? item.sort : fallbackSort,
    }
  }

  const closeSlashPopover = () => {
    slashOpen.value = false
    slashActiveIndex.value = 0
    slashQuery.value = ''
    slashRange.value = null
  }

  const findSlashRange = (text: string, caret: number): { start: number; end: number; query: string } | null => {
    if (caret < 0 || caret > text.length) return null
    let idx = caret - 1
    while (idx >= 0 && text[idx] !== '\n') {
      if (text[idx] === '/') {
        const prev = idx === 0 ? ' ' : text[idx - 1]
        if (idx === 0 || /\s/.test(prev)) {
          const segment = text.slice(idx + 1, caret)
          if (!/\s/.test(segment)) {
            return { start: idx, end: caret, query: segment }
          }
        }
        return null
      }
      if (/\s/.test(text[idx])) break
      idx -= 1
    }
    return null
  }

  const updateSlashState = (text: string, caret: number) => {
    const range = findSlashRange(text, caret)
    if (!range) {
      closeSlashPopover()
      return
    }
    const previousQuery = slashQuery.value
    const wasOpen = slashOpen.value
    slashRange.value = { start: range.start, end: range.end }
    slashQuery.value = range.query
    slashOpen.value = true
    if (!wasOpen || range.query !== previousQuery) {
      slashActiveIndex.value = 0
    } else {
      const maxIndex = Math.max(0, filteredSlashCommands.value.length - 1)
      slashActiveIndex.value = Math.min(slashActiveIndex.value, maxIndex)
    }
  }

  const executeSlashAction = (action?: SlashActionType) => {
    if (!action) return
    switch (action) {
      case 'new_conversation':
        params.createNewConversation()
        break
      case 'clear_conversation':
        params.clearConversation()
        break
      case 'toggle_rag':
        params.toggleRAG()
        break
      case 'toggle_tools':
        params.toggleTools()
        break
      case 'open_tool_config':
        params.emitOpenToolConfig()
        break
    }
  }

  const applySlashCommand = (command: SlashCommandItem) => {
    const range = slashRange.value
    const original = params.getInputMessage() || ''
    if (!range) return

    if (command.type === 'action') {
      const before = original.slice(0, range.start)
      const after = original.slice(range.end)
      const newValue = `${before}${after}`.replace(/^\s+/, '')
      params.onInputValueChange(newValue)
      closeSlashPopover()
      executeSlashAction(command.action)
      return
    }

    const afterSlash = original.slice(range.end).trimStart()
    const template = command.template || ''
    const rendered = template.includes('{{input}}')
      ? template.split('{{input}}').join(afterSlash)
      : [template, afterSlash].filter(Boolean).join(' ')
    params.onInputValueChange(`${original.slice(0, range.start)}${rendered}`)
    closeSlashPopover()
    if (command.auto_send) {
      params.emitSend()
    }
  }

  const loadSlashCommands = async () => {
    try {
      const items = await invoke<Array<{ key: string; value: string }>>('get_config', {
        request: { category: SLASH_COMMANDS_CATEGORY, key: SLASH_COMMANDS_KEY },
      })
      if (!items || items.length === 0 || !items[0]?.value) {
        globalSlashCommands.value = []
        conversationSlashCommands.value = {}
        return
      }
      const parsed = JSON.parse(items[0].value)
      if (Array.isArray(parsed)) {
        globalSlashCommands.value = parsed
          .filter((item) => item && typeof item.name === 'string')
          .map((item, idx) => normalizeSlashCommand(item, 'global', idx))
        conversationSlashCommands.value = {}
        return
      }
      const rawGlobal = Array.isArray(parsed?.global) ? parsed.global : []
      const rawConversations = parsed?.conversations && typeof parsed.conversations === 'object'
        ? parsed.conversations
        : {}
      globalSlashCommands.value = rawGlobal
        .filter((item: any) => item && typeof item.name === 'string')
        .map((item: any, idx: number) => normalizeSlashCommand(item, 'global', idx))
      const convStore: Record<string, SlashCommandItem[]> = {}
      Object.entries(rawConversations).forEach(([key, list]) => {
        if (!Array.isArray(list)) return
        convStore[key] = list
          .filter((item: any) => item && typeof item.name === 'string')
          .map((item: any, idx: number) => normalizeSlashCommand(item, 'conversation', idx))
      })
      conversationSlashCommands.value = convStore
    } catch (error) {
      console.warn('[useInputSlashCommands] Failed to load slash commands:', error)
      globalSlashCommands.value = []
      conversationSlashCommands.value = {}
    }
  }

  const saveSlashCommands = async () => {
    try {
      const value = JSON.stringify({
        version: 2,
        global: globalSlashCommands.value,
        conversations: conversationSlashCommands.value,
      })
      await invoke('set_config', {
        category: SLASH_COMMANDS_CATEGORY,
        key: SLASH_COMMANDS_KEY,
        value,
      })
    } catch (error) {
      console.error('[useInputSlashCommands] Failed to save slash commands:', error)
      dialog.toast.error('保存 Slash 命令失败')
    }
  }

  const resetSlashForm = () => {
    editingSlashId.value = null
    slashFormError.value = ''
    newSlashCommand.value = {
      id: '',
      name: '',
      description: '',
      type: 'prompt',
      template: '{{input}}',
      action: 'new_conversation',
      auto_send: false,
      enabled: true,
      scope: slashManagerScope.value,
      sort: 0,
    }
  }

  const addCustomSlashCommand = async () => {
    try {
      slashFormError.value = ''
      const name = newSlashCommand.value.name.trim().replace(/^\//, '')
      if (!name) {
        slashFormError.value = '命令名不能为空'
        dialog.toast.error('命令名不能为空')
        return
      }
      const scope = newSlashCommand.value.scope === 'conversation' ? 'conversation' : 'global'
      if (scope === 'conversation' && !conversationScopeEnabled.value) {
        slashFormError.value = '当前没有会话，无法创建会话级命令'
        dialog.toast.error('当前没有会话，无法创建会话级命令')
        return
      }

      const targetList = getScopeCommandsMutable(scope)
      const exists = [...builtinSlashCommands.value, ...targetList].some(
        (command) => command.name.toLowerCase() === name.toLowerCase(),
      )
      if (exists) {
        slashFormError.value = `命令 /${name} 已存在`
        dialog.toast.error(`命令 /${name} 已存在`)
        return
      }
      if (newSlashCommand.value.type === 'prompt' && !newSlashCommand.value.template?.trim()) {
        slashFormError.value = '提示词模板不能为空'
        dialog.toast.error('提示词模板不能为空')
        return
      }
      if (newSlashCommand.value.type === 'action' && !newSlashCommand.value.action) {
        slashFormError.value = '请选择功能动作'
        dialog.toast.error('请选择功能动作')
        return
      }

      targetList.push({
        id: generateCommandId(),
        name,
        description: newSlashCommand.value.description?.trim() || '',
        type: newSlashCommand.value.type,
        template: newSlashCommand.value.type === 'prompt' ? newSlashCommand.value.template || '' : '',
        action: newSlashCommand.value.type === 'action' ? newSlashCommand.value.action : undefined,
        auto_send: !!newSlashCommand.value.auto_send,
        enabled: true,
        scope,
        sort: targetList.length,
      })
      await saveSlashCommands()
      resetSlashForm()
      dialog.toast.success('Slash 命令已添加')
    } catch (error: any) {
      const message = `添加命令失败: ${String(error)}`
      slashFormError.value = message
      dialog.toast.error(message)
    }
  }

  const startEditCommand = (command: SlashCommandItem) => {
    if (command.is_builtin) return
    editingSlashId.value = command.id
    newSlashCommand.value = {
      id: command.id,
      name: command.name,
      description: command.description || '',
      type: command.type,
      template: command.template || '',
      action: command.action || 'new_conversation',
      auto_send: !!command.auto_send,
      enabled: command.enabled !== false,
      scope: command.scope || slashManagerScope.value,
      sort: command.sort ?? 0,
    }
  }

  const normalizeSortForScope = (scope: SlashCommandScope) => {
    const list = getScopeCommandsMutable(scope)
    list.sort(sortByOrder)
    list.forEach((item, idx) => {
      item.sort = idx
    })
  }

  const saveEditedSlashCommand = async () => {
    try {
      slashFormError.value = ''
      const editId = editingSlashId.value
      if (!editId) return
      const name = newSlashCommand.value.name.trim().replace(/^\//, '')
      if (!name) {
        slashFormError.value = '命令名不能为空'
        dialog.toast.error('命令名不能为空')
        return
      }
      const targetScope = newSlashCommand.value.scope === 'conversation' ? 'conversation' : 'global'
      if (targetScope === 'conversation' && !conversationScopeEnabled.value) {
        slashFormError.value = '当前没有会话，无法保存会话级命令'
        dialog.toast.error('当前没有会话，无法保存会话级命令')
        return
      }
      const targetListForConflict = getScopeCommandsMutable(targetScope)
      const existingConflict = [...builtinSlashCommands.value, ...targetListForConflict].some((command) => {
        if (command.id === editId) return false
        return command.name.toLowerCase() === name.toLowerCase()
      })
      if (existingConflict) {
        slashFormError.value = `命令 /${name} 已存在`
        dialog.toast.error(`命令 /${name} 已存在`)
        return
      }
      globalSlashCommands.value = globalSlashCommands.value.filter((command) => command.id !== editId)
      Object.keys(conversationSlashCommands.value).forEach((key) => {
        conversationSlashCommands.value[key] = (conversationSlashCommands.value[key] || []).filter(
          (command) => command.id !== editId,
        )
      })
      const targetList = getScopeCommandsMutable(targetScope)
      targetList.push({
        id: editId,
        name,
        description: newSlashCommand.value.description?.trim() || '',
        type: newSlashCommand.value.type,
        template: newSlashCommand.value.type === 'prompt' ? newSlashCommand.value.template || '' : '',
        action: newSlashCommand.value.type === 'action' ? newSlashCommand.value.action : undefined,
        auto_send: !!newSlashCommand.value.auto_send,
        enabled: newSlashCommand.value.enabled !== false,
        scope: targetScope,
        sort: targetList.length,
      })
      normalizeSortForScope(targetScope)
      await saveSlashCommands()
      resetSlashForm()
      dialog.toast.success('Slash 命令已更新')
    } catch (error: any) {
      const message = `保存命令失败: ${String(error)}`
      slashFormError.value = message
      dialog.toast.error(message)
    }
  }

  const cancelEditCommand = () => {
    resetSlashForm()
  }

  const deleteSlashCommand = async (id: string) => {
    if (editingSlashId.value === id) {
      resetSlashForm()
    }
    if (slashManagerScope.value === 'conversation') {
      const key = conversationScopeKey.value
      conversationSlashCommands.value[key] = (conversationSlashCommands.value[key] || []).filter(
        (command) => command.id !== id,
      )
    } else {
      globalSlashCommands.value = globalSlashCommands.value.filter((command) => command.id !== id)
    }
    normalizeSortForScope(slashManagerScope.value)
    await saveSlashCommands()
  }

  const toggleCommandEnabled = async (command: SlashCommandItem, enabled: boolean) => {
    if (command.is_builtin) return
    const target = getScopeCommandsMutable(slashManagerScope.value).find((item) => item.id === command.id)
    if (!target) return
    target.enabled = enabled
    await saveSlashCommands()
  }

  const onCommandEnabledChange = async (command: SlashCommandItem, event: Event) => {
    const target = event.target as HTMLInputElement | null
    await toggleCommandEnabled(command, !!target?.checked)
  }

  const onDragSortEnd = async () => {
    normalizeSortForScope(slashManagerScope.value)
    await saveSlashCommands()
  }

  const exportSlashCommands = () => {
    const scope = slashManagerScope.value
    const payload = {
      version: 1,
      scope,
      commands: scopeCommandsForManager.value.filter((command) => !command.is_builtin),
    }
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = `slash-commands-${scope}-${new Date().toISOString().slice(0, 10)}.json`
    anchor.click()
    URL.revokeObjectURL(url)
  }

  const triggerImportSlashCommands = () => {
    importSlashInputRef.value?.click()
  }

  const onImportSlashFileChange = async (event: Event) => {
    const input = event.target as HTMLInputElement
    const file = input.files?.[0]
    if (!file) return
    try {
      const text = await file.text()
      const parsed = JSON.parse(text)
      const importedRaw = Array.isArray(parsed) ? parsed : parsed?.commands
      if (!Array.isArray(importedRaw)) {
        dialog.toast.error('导入文件格式无效')
        return
      }
      const scope = slashManagerScope.value
      const list = getScopeCommandsMutable(scope)
      const byName = new Map<string, SlashCommandItem>()
      list.forEach((item) => byName.set(item.name.toLowerCase(), item))
      const plan = importedRaw
        .map((item: any, idx: number) => {
          if (!item || typeof item.name !== 'string') return null
          const normalized = normalizeSlashCommand(item, scope, list.length + idx)
          normalized.scope = scope
          normalized.is_builtin = false
          return normalized
        })
        .filter(Boolean) as SlashCommandItem[]

      const toAdd: string[] = []
      const toUpdate: string[] = []
      plan.forEach((command) => {
        const existing = byName.get(command.name.toLowerCase())
        if (existing) toUpdate.push(`/${command.name}`)
        else toAdd.push(`/${command.name}`)
      })

      const previewParts: string[] = [
        `作用域: ${scope === 'conversation' ? '会话' : '全局'}`,
        `新增: ${toAdd.length}`,
        `覆盖: ${toUpdate.length}`,
      ]
      if (toAdd.length > 0) {
        previewParts.push(`新增命令: ${toAdd.slice(0, 8).join(', ')}${toAdd.length > 8 ? ' ...' : ''}`)
      }
      if (toUpdate.length > 0) {
        previewParts.push(`覆盖命令: ${toUpdate.slice(0, 8).join(', ')}${toUpdate.length > 8 ? ' ...' : ''}`)
      }

      const confirmed = await dialog.confirm({
        title: '导入 Slash 命令',
        message: `${previewParts.join('\n')}\n\n确认执行导入？`,
        variant: 'warning',
      })
      if (!confirmed) return

      plan.forEach((normalized) => {
        const existing = byName.get(normalized.name.toLowerCase())
        if (existing) {
          existing.description = normalized.description
          existing.type = normalized.type
          existing.template = normalized.template
          existing.action = normalized.action
          existing.auto_send = normalized.auto_send
          existing.enabled = normalized.enabled
        } else {
          list.push(normalized)
          byName.set(normalized.name.toLowerCase(), normalized)
        }
      })
      normalizeSortForScope(scope)
      await saveSlashCommands()
      dialog.toast.success(`Slash 命令导入完成（新增 ${toAdd.length}，覆盖 ${toUpdate.length}）`)
    } catch (error) {
      console.error('[useInputSlashCommands] Failed to import slash commands:', error)
      dialog.toast.error('导入 Slash 命令失败')
    } finally {
      input.value = ''
    }
  }

  const openSlashManager = async () => {
    await loadSlashCommands()
    if (slashManagerScope.value === 'conversation' && !conversationScopeEnabled.value) {
      slashManagerScope.value = 'global'
    }
    resetSlashForm()
    showSlashManager.value = true
  }

  const closeSlashManager = () => {
    showSlashManager.value = false
  }

  watch(
    () => filteredSlashCommands.value.length,
    (len) => {
      if (len <= 0) {
        slashActiveIndex.value = 0
        return
      }
      if (slashActiveIndex.value >= len) {
        slashActiveIndex.value = len - 1
      }
    },
  )

  watch(slashManagerScope, (scope) => {
    if (scope === 'conversation' && !conversationScopeEnabled.value) {
      slashManagerScope.value = 'global'
      return
    }
    slashFormError.value = ''
    newSlashCommand.value.scope = scope
  })

  watch(
    () => [
      newSlashCommand.value.name,
      newSlashCommand.value.description,
      newSlashCommand.value.type,
      newSlashCommand.value.template,
      newSlashCommand.value.action,
      newSlashCommand.value.scope,
    ],
    () => {
      if (slashFormError.value) {
        slashFormError.value = ''
      }
    },
  )

  watch(
    () => params.conversationId.value,
    () => {
      if (slashManagerScope.value === 'conversation' && !conversationScopeEnabled.value) {
        slashManagerScope.value = 'global'
      }
    },
  )

  return {
    addCustomSlashCommand,
    applySlashCommand,
    cancelEditCommand,
    closeSlashManager,
    closeSlashPopover,
    conversationScopeEnabled,
    conversationScopeKey,
    deleteSlashCommand,
    editingSlashId,
    filteredSlashCommands,
    getActionLabel,
    getScopeLabel,
    importSlashInputRef,
    loadSlashCommands,
    managerBuiltinCommands,
    managerCustomCommands,
    newSlashCommand,
    onCommandEnabledChange,
    onDragSortEnd,
    onImportSlashFileChange,
    openSlashManager,
    exportSlashCommands,
    saveEditedSlashCommand,
    showSlashManager,
    slashActiveIndex,
    slashFormError,
    slashManagerScope,
    slashOpen,
    startEditCommand,
    triggerImportSlashCommands,
    updateSlashState,
  }
}
