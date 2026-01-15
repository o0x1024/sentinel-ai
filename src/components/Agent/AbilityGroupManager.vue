<template>
  <div :class="['ability-group-manager', { 'fullscreen': isFullscreen }]">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold flex items-center gap-2">
        <i class="fas fa-layer-group text-primary"></i>
        {{ t('agent.abilityGroupManagement') }}
      </h3>
      <div class="flex items-center gap-2">
        <button 
          @click="$emit('toggle-fullscreen')" 
          class="btn btn-sm btn-ghost btn-circle"
          :title="isFullscreen ? t('common.exitFullscreen') : t('common.fullscreen')"
        >
          <i :class="isFullscreen ? 'fas fa-compress' : 'fas fa-expand'"></i>
        </button>
        <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Create / Edit Form - Left-Right Layout -->
    <div v-if="editingGroup" class="card bg-base-200 p-4 mb-4">
      <div class="grid grid-cols-1 lg:grid-cols-5 gap-4">
        <!-- Left Panel: Content Input (60%) -->
        <div class="lg:col-span-3 space-y-3">
          <div class="text-sm font-medium text-base-content/70 mb-2">
            <i class="fas fa-edit mr-1"></i>
            {{ t('agent.leftPanel') }}
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.groupName') }}</span>
            </label>
            <input 
              v-model="editingGroup.name" 
              type="text" 
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.groupNamePlaceholder')"
            />
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.groupDescription') }}</span>
            </label>
            <input 
              v-model="editingGroup.description" 
              type="text" 
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.groupDescriptionPlaceholder')"
            />
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.groupInstructions') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('agent.instructionsHint') }}</span>
            </label>
            <textarea 
              v-model="editingGroup.instructions" 
              class="textarea textarea-bordered w-full h-28 text-sm"
              :placeholder="t('agent.instructionsPlaceholder')"
            ></textarea>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.additionalNotes') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('agent.additionalNotesHint') }}</span>
            </label>
            <textarea 
              v-model="editingGroup.additional_notes" 
              class="textarea textarea-bordered w-full h-28 text-sm"
              :placeholder="t('agent.additionalNotesPlaceholder')"
            ></textarea>
          </div>

          <div class="space-y-2 pt-2">
            <button 
              @click="generateWithAI" 
              class="btn btn-sm btn-primary btn-outline gap-1 w-full"
              :disabled="!canUseAI || aiGenerating"
              :class="{ 'btn-disabled': !canUseAI }"
              :title="!canUseAI ? t('agent.selectToolsOrAddContent') : ''"
            >
              <span v-if="aiGenerating" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-magic"></i>
              {{ aiButtonText }}
            </button>
            <div v-if="!canUseAI" class="text-xs text-warning text-center">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('agent.aiGenerateHint') }}
            </div>
          </div>

          <div class="flex justify-end gap-2 pt-2 border-t border-base-300">
            <button @click="cancelEdit" class="btn btn-sm btn-ghost">
              {{ t('common.cancel') }}
            </button>
            <button @click="saveGroup" class="btn btn-sm btn-primary" :disabled="!canSave">
              <i class="fas fa-save"></i>
              {{ isNewGroup ? t('common.create') : t('common.save') }}
            </button>
          </div>
        </div>

        <!-- Right Panel: Tool Selection (40%) -->
        <div class="lg:col-span-2 space-y-3">
          <div class="text-sm font-medium text-base-content/70 mb-2">
            <i class="fas fa-tools mr-1"></i>
            {{ t('agent.rightPanel') }}
          </div>

          <div class="form-control">
            <input 
              v-model="toolSearchQuery"
              type="text" 
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.searchTools')"
            />
          </div>

          <div class="form-control">
            <select 
              v-model="selectedCategory"
              class="select select-sm select-bordered w-full"
            >
              <option value="">{{ t('agent.filterByCategory') }}</option>
              <option v-for="cat in toolCategories" :key="cat" :value="cat">
                {{ getCategoryDisplayName(cat) }}
              </option>
            </select>
          </div>

          <div class="flex items-center justify-between text-xs text-base-content/60">
            <span>{{ selectedToolCount }} {{ t('agent.toolsSelected') }}</span>
            <div class="flex gap-1">
              <button 
                v-if="filteredTools.length > 0 && !isAllFilteredSelected"
                @click="selectAllFilteredTools" 
                class="btn btn-xs btn-ghost"
              >
                {{ t('agent.selectAll') }}
              </button>
              <button 
                v-if="selectedToolCount > 0"
                @click="clearToolSelection" 
                class="btn btn-xs btn-ghost"
              >
                {{ t('agent.clearSelection') }}
              </button>
            </div>
          </div>

          <div class="border border-base-300 rounded-lg p-2 max-h-96 overflow-y-auto bg-base-100">
            <div v-if="loadingTools" class="flex justify-center py-4">
              <span class="loading loading-spinner loading-sm"></span>
            </div>
            <div v-else class="space-y-1">
              <label 
                v-for="tool in filteredTools" 
                :key="tool.id"
                class="flex items-center gap-2 p-2 hover:bg-base-200 rounded cursor-pointer transition-colors"
              >
                <input 
                  type="checkbox"
                  :value="tool.id"
                  v-model="editingGroup.tool_ids"
                  class="checkbox checkbox-xs checkbox-primary"
                />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium truncate">{{ tool.name }}</div>
                  <div class="text-xs text-base-content/60 truncate">{{ tool.description }}</div>
                </div>
                <span class="badge badge-xs flex-shrink-0" :class="getCategoryBadgeClass(tool.category)">
                  {{ getCategoryDisplayName(tool.category) }}
                </span>
              </label>
              <div v-if="filteredTools.length === 0" class="text-center py-4 text-base-content/60 text-sm">
                {{ t('agent.noToolsAvailable') }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Group List -->
    <div class="space-y-2">
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm text-base-content/60">
          {{ groups.length }} {{ t('agent.groups') }}
        </span>
        <button @click="startCreate" class="btn btn-sm btn-primary btn-outline">
          <i class="fas fa-plus"></i>
          {{ t('agent.createGroup') }}
        </button>
      </div>

      <div v-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-md"></span>
      </div>

      <div v-else-if="groups.length === 0" class="text-center py-8 text-base-content/60">
        <i class="fas fa-inbox text-3xl mb-2"></i>
        <p>{{ t('agent.noAbilityGroups') }}</p>
        <p class="text-sm mt-1">{{ t('agent.createFirstGroup') }}</p>
      </div>

      <div v-else class="space-y-2">
        <div 
          v-for="group in groups" 
          :key="group.id"
          class="card bg-base-200 p-3 hover:bg-base-300 transition-colors"
        >
          <div class="flex items-start justify-between">
            <div class="flex-1 min-w-0">
              <div class="font-medium">{{ group.name }}</div>
              <div class="text-sm text-base-content/60 truncate">{{ group.description }}</div>
              <div class="flex items-center gap-2 mt-1 flex-wrap">
                <span class="badge badge-sm badge-ghost">
                  {{ group.tool_ids?.length || 0 }} {{ t('agent.tools') }}
                </span>
                <span v-if="group.instructions" class="badge badge-sm badge-info badge-outline">
                  {{ t('agent.hasInstructions') }}
                </span>
                <span v-if="group.additional_notes" class="badge badge-sm badge-warning badge-outline">
                  {{ t('agent.hasAdditionalNotes') }}
                </span>
              </div>
            </div>
            <div class="flex gap-1 ml-2">
              <button @click="startEdit(group)" class="btn btn-xs btn-ghost btn-circle">
                <i class="fas fa-edit"></i>
              </button>
              <button @click="confirmDelete(group)" class="btn btn-xs btn-ghost btn-circle text-error">
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'

interface AbilityGroup {
  id: string
  name: string
  description: string
  instructions: string
  additional_notes: string
  tool_ids: string[]
  created_at?: string
  updated_at?: string
}

interface ToolMetadata {
  id: string
  name: string
  description: string
  category: string
}

interface Props {
  isFullscreen?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  isFullscreen: false
})

const emit = defineEmits<{
  'close': []
  'changed': []
  'toggle-fullscreen': []
}>()

const { t } = useI18n()

const groups = ref<AbilityGroup[]>([])
const availableTools = ref<ToolMetadata[]>([])
const filteredTools = ref<ToolMetadata[]>([])
const loading = ref(false)
const loadingTools = ref(false)
const editingGroup = ref<AbilityGroup | null>(null)
const isNewGroup = ref(false)
const aiGenerating = ref(false)
const generatedContent = ref('')
const toolSearchQuery = ref('')
const selectedCategory = ref('')

const isFullscreen = computed(() => props.isFullscreen)

const selectedToolCount = computed(() => editingGroup.value?.tool_ids?.length || 0)

const isAllFilteredSelected = computed(() => {
  if (!editingGroup.value || filteredTools.value.length === 0) return false
  return filteredTools.value.every(tool => editingGroup.value?.tool_ids.includes(tool.id))
})

const hasExistingContent = computed(() => {
  if (!editingGroup.value) return false
  return !!(
    editingGroup.value.name.trim() || 
    editingGroup.value.description.trim() || 
    editingGroup.value.instructions.trim() || 
    editingGroup.value.additional_notes.trim()
  )
})

const canUseAI = computed(() => {
  return selectedToolCount.value > 0 || hasExistingContent.value
})

const aiButtonText = computed(() => {
  if (aiGenerating.value) return t('agent.aiGenerating')
  return hasExistingContent.value ? t('agent.aiExpand') : t('agent.aiGenerate')
})

const canSave = computed(() => {
  if (!editingGroup.value) return false
  return editingGroup.value.name.trim().length > 0
})

const toolCategories = computed(() => {
  const categories = new Set<string>()
  availableTools.value.forEach(tool => {
    if (tool.category) categories.add(tool.category)
  })
  return Array.from(categories).sort()
})

const filterTools = () => {
  let result = availableTools.value

  if (toolSearchQuery.value.trim()) {
    const query = toolSearchQuery.value.toLowerCase()
    result = result.filter(tool => 
      tool.name.toLowerCase().includes(query) || 
      tool.description.toLowerCase().includes(query)
    )
  }

  if (selectedCategory.value) {
    result = result.filter(tool => tool.category === selectedCategory.value)
  }

  filteredTools.value = result
}

const clearToolSelection = () => {
  if (editingGroup.value) {
    editingGroup.value.tool_ids = []
  }
}

const selectAllFilteredTools = () => {
  if (!editingGroup.value) return
  const currentIds = new Set(editingGroup.value.tool_ids)
  filteredTools.value.forEach(tool => currentIds.add(tool.id))
  editingGroup.value.tool_ids = Array.from(currentIds)
}

const loadGroups = async () => {
  loading.value = true
  try {
    groups.value = await invoke<AbilityGroup[]>('list_ability_groups_full')
  } catch (error) {
    console.error('Failed to load ability groups:', error)
  } finally {
    loading.value = false
  }
}

const loadTools = async () => {
  loadingTools.value = true
  try {
    availableTools.value = await invoke<ToolMetadata[]>('get_all_tool_metadata')
  } catch (error) {
    console.error('Failed to load tools:', error)
  } finally {
    loadingTools.value = false
  }
}

const startCreate = () => {
  isNewGroup.value = true
  editingGroup.value = {
    id: '',
    name: '',
    description: '',
    instructions: '',
    additional_notes: '',
    tool_ids: [],
  }
}

const startEdit = (group: AbilityGroup) => {
  isNewGroup.value = false
  editingGroup.value = { 
    ...group, 
    tool_ids: [...(group.tool_ids || [])],
    additional_notes: group.additional_notes || ''
  }
}

const cancelEdit = () => {
  editingGroup.value = null
  isNewGroup.value = false
}

const saveGroup = async () => {
  if (!editingGroup.value || !canSave.value) return

  try {
    if (isNewGroup.value) {
      await invoke('create_ability_group', {
        payload: {
          name: editingGroup.value.name,
          description: editingGroup.value.description,
          instructions: editingGroup.value.instructions,
          additional_notes: editingGroup.value.additional_notes,
          tool_ids: editingGroup.value.tool_ids,
        }
      })
    } else {
      await invoke('update_ability_group', {
        id: editingGroup.value.id,
        payload: {
          name: editingGroup.value.name,
          description: editingGroup.value.description,
          instructions: editingGroup.value.instructions,
          additional_notes: editingGroup.value.additional_notes,
          tool_ids: editingGroup.value.tool_ids,
        }
      })
    }
    
    editingGroup.value = null
    isNewGroup.value = false
    await loadGroups()
    emit('changed')
  } catch (error) {
    console.error('Failed to save ability group:', error)
  }
}

const confirmDelete = async (group: AbilityGroup) => {
  if (!confirm(t('agent.confirmDeleteGroup', { name: group.name }))) return
  
  try {
    await invoke('delete_ability_group', { id: group.id })
    await loadGroups()
    emit('changed')
  } catch (error) {
    console.error('Failed to delete ability group:', error)
  }
}

const generateWithAI = async () => {
  if (!editingGroup.value) return

  // Check if we can use AI (should be prevented by button disabled state, but double check)
  if (!canUseAI.value) {
    alert(t('agent.selectToolsOrAddContent'))
    return
  }

  aiGenerating.value = true
  generatedContent.value = ''
  
  const selectedTools = availableTools.value.filter(t => editingGroup.value?.tool_ids.includes(t.id))
  const toolsContext = selectedTools.map(t => `- ${t.name}: ${t.description}`).join('\n')
  
  // Check if there's existing content
  const hasExistingContent = 
    editingGroup.value.name.trim() || 
    editingGroup.value.description.trim() || 
    editingGroup.value.instructions.trim() || 
    editingGroup.value.additional_notes.trim()

  let prompt = ''
  
  if (hasExistingContent) {
    // Expand mode: enhance existing content
    prompt = `Based on the following tools and existing content, please enhance and expand the ability group configuration.

Tools:
${toolsContext}

Existing Content:
- Name: ${editingGroup.value.name || '(empty)'}
- Description: ${editingGroup.value.description || '(empty)'}
- Instructions: ${editingGroup.value.instructions || '(empty)'}
- Additional Notes: ${editingGroup.value.additional_notes || '(empty)'}

Please:
1. Keep the existing content that is good
2. Enhance empty or incomplete fields
3. Expand instructions with more details based on the tools
4. Add relevant additional notes if missing

Return ONLY a JSON object with the following structure:
{
  "name": "Enhanced Group Name",
  "description": "Enhanced short description (one sentence)",
  "instructions": "Enhanced and detailed instructions on when and how to use this group",
  "additional_notes": "Enhanced additional context, limitations, best practices, or warnings"
}
Ensure the content is in the same language as the existing content (or Chinese if unsure).
`
  } else {
    // Generate mode: create from scratch
    prompt = `Based on the following tools, please generate a name, description, detailed instructions, and additional notes for an ability group.
Tools:
${toolsContext}

Return ONLY a JSON object with the following structure:
{
  "name": "Group Name",
  "description": "Short description (one sentence)",
  "instructions": "Detailed instructions on when and how to use this group",
  "additional_notes": "Additional context, limitations, best practices, or warnings"
}
Ensure the content is in the same language as the tool descriptions (or Chinese if unsure).
`
  }

  const streamId = crypto.randomUUID()
  
  let unlistenDelta: (() => void) | undefined
  let unlistenComplete: (() => void) | undefined
  let unlistenError: (() => void) | undefined

  try {
    unlistenDelta = await listen(`plugin_gen_delta`, (event) => {
      const payload = event.payload as any
      if (payload.stream_id === streamId) {
        generatedContent.value += payload.delta
      }
    })

    unlistenComplete = await listen(`plugin_gen_complete`, (event) => {
      const payload = event.payload as any
      if (payload.stream_id === streamId) {
        try {
          let jsonStr = generatedContent.value.trim()
          // Extract JSON from markdown code blocks
          const jsonMatch = jsonStr.match(/```(?:json)?\s*(\{[\s\S]*?\})\s*```/)
          if (jsonMatch) {
            jsonStr = jsonMatch[1]
          } else if (jsonStr.startsWith('```')) {
             jsonStr = jsonStr.replace(/^```(?:json)?/, '').replace(/```$/, '')
          }
          
          const data = JSON.parse(jsonStr)
          if (editingGroup.value) {
            editingGroup.value.name = data.name
            editingGroup.value.description = data.description
            editingGroup.value.instructions = data.instructions
            editingGroup.value.additional_notes = data.additional_notes || ''
          }
        } catch (e) {
          console.error('Failed to parse AI response', e)
        } finally {
          aiGenerating.value = false
          if (unlistenDelta) unlistenDelta()
          if (unlistenComplete) unlistenComplete()
          if (unlistenError) unlistenError()
        }
      }
    })
    
    unlistenError = await listen(`plugin_gen_error`, (event) => {
        const payload = event.payload as any
        if (payload.stream_id === streamId) {
            console.error('AI Generation Error:', payload.error)
            aiGenerating.value = false
            if (unlistenDelta) unlistenDelta()
            if (unlistenComplete) unlistenComplete()
            if (unlistenError) unlistenError()
        }
    })

    const systemPrompt = hasExistingContent
      ? "You are a helpful assistant that enhances and expands configuration for AI agent ability groups. Keep good existing content, enhance incomplete parts, and add missing details. Return valid JSON only."
      : "You are a helpful assistant that generates configuration for AI agent ability groups. Return valid JSON only."

    await invoke('generate_plugin_stream', {
      request: {
        stream_id: streamId,
        message: prompt,
        system_prompt: systemPrompt,
        service_name: "default"
      }
    })
  } catch (e) {
    console.error(e)
    aiGenerating.value = false
    if (unlistenDelta) unlistenDelta()
    if (unlistenComplete) unlistenComplete()
    if (unlistenError) unlistenError()
  }
}

const getCategoryDisplayName = (category: string) => {
  const nameMap: Record<string, string> = {
    'network': '网络',
    'security': '安全',
    'data': '数据',
    'ai': 'AI',
    'system': '系统',
    'mcp': 'MCP',
    'plugin': '插件',
    'workflow': '工作流',
    'browser': '浏览器',
  }
  return nameMap[category.toLowerCase()] || category
}

const getCategoryBadgeClass = (category: string) => {
  const map: Record<string, string> = {
    'network': 'badge-info',
    'security': 'badge-error',
    'data': 'badge-success',
    'ai': 'badge-warning',
    'system': 'badge-neutral',
    'mcp': 'badge-primary',
    'plugin': 'badge-secondary',
    'workflow': 'badge-accent',
    'browser': 'badge-primary',
  }
  return map[category.toLowerCase()] || 'badge-ghost'
}

watch([toolSearchQuery, selectedCategory], () => {
  filterTools()
})

watch(availableTools, () => {
  filterTools()
})

onMounted(() => {
  loadGroups()
  loadTools()
})
</script>

<style scoped>
.ability-group-manager {
  max-height: 85vh;
  overflow-y: auto;
}

/* 全屏模式 */
.ability-group-manager.fullscreen {
  max-height: calc(100vh - 2rem);
  height: calc(100vh - 2rem);
}

/* 响应式布局优化 */
@media (max-width: 1024px) {
  .ability-group-manager :deep(.grid-cols-5) {
    grid-template-columns: 1fr;
  }
}
</style>

