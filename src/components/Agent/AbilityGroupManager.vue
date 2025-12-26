<template>
  <div class="ability-group-manager">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold flex items-center gap-2">
        <i class="fas fa-layer-group text-primary"></i>
        {{ t('agent.abilityGroupManagement') }}
      </h3>
      <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- Create / Edit Form -->
    <div v-if="editingGroup" class="card bg-base-200 p-4 mb-4">
      <div class="space-y-3">
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
            class="textarea textarea-bordered w-full h-32 text-sm"
            :placeholder="t('agent.instructionsPlaceholder')"
          ></textarea>
        </div>

        <div class="form-control">
          <label class="label py-1">
            <span class="label-text flex items-center gap-2">
              {{ t('agent.selectTools') }}
              <button 
                v-if="selectedToolCount > 0"
                @click="generateWithAI" 
                class="btn btn-xs btn-primary btn-outline gap-1"
                :disabled="aiGenerating"
              >
                <span v-if="aiGenerating" class="loading loading-spinner loading-xs"></span>
                <i v-else class="fas fa-magic"></i>
                {{ aiGenerating ? t('agent.aiGenerating') : t('agent.aiGenerate') }}
              </button>
            </span>
            <span class="label-text-alt">{{ selectedToolCount }} {{ t('agent.selected') }}</span>
          </label>
          <div class="border border-base-300 rounded-lg p-2 max-h-48 overflow-y-auto">
            <div v-if="loadingTools" class="flex justify-center py-4">
              <span class="loading loading-spinner loading-sm"></span>
            </div>
            <div v-else class="space-y-1">
              <label 
                v-for="tool in availableTools" 
                :key="tool.id"
                class="flex items-center gap-2 p-1 hover:bg-base-200 rounded cursor-pointer"
              >
                <input 
                  type="checkbox"
                  :value="tool.id"
                  v-model="editingGroup.tool_ids"
                  class="checkbox checkbox-xs checkbox-primary"
                />
                <span class="text-sm flex-1 truncate">{{ tool.name }}</span>
                <span class="badge badge-xs" :class="getCategoryBadgeClass(tool.category)">
                  {{ getCategoryDisplayName(tool.category) }}
                </span>
              </label>
              <div v-if="availableTools.length === 0" class="text-center py-2 text-base-content/60 text-sm">
                {{ t('agent.noToolsAvailable') }}
              </div>
            </div>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button @click="cancelEdit" class="btn btn-sm btn-ghost">
            {{ t('common.cancel') }}
          </button>
          <button @click="saveGroup" class="btn btn-sm btn-primary" :disabled="!canSave">
            <i class="fas fa-save"></i>
            {{ isNewGroup ? t('common.create') : t('common.save') }}
          </button>
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
              <div class="flex items-center gap-2 mt-1">
                <span class="badge badge-sm badge-ghost">
                  {{ group.tool_ids?.length || 0 }} {{ t('agent.tools') }}
                </span>
                <span v-if="group.instructions" class="badge badge-sm badge-info badge-outline">
                  {{ t('agent.hasInstructions') }}
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
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'

interface AbilityGroup {
  id: string
  name: string
  description: string
  instructions: string
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

const emit = defineEmits<{
  'close': []
  'changed': []
}>()

const { t } = useI18n()

const groups = ref<AbilityGroup[]>([])
const availableTools = ref<ToolMetadata[]>([])
const loading = ref(false)
const loadingTools = ref(false)
const editingGroup = ref<AbilityGroup | null>(null)
const isNewGroup = ref(false)
const aiGenerating = ref(false)
const generatedContent = ref('')

const selectedToolCount = computed(() => editingGroup.value?.tool_ids?.length || 0)

const canSave = computed(() => {
  if (!editingGroup.value) return false
  return editingGroup.value.name.trim().length > 0
})

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
    tool_ids: [],
  }
}

const startEdit = (group: AbilityGroup) => {
  isNewGroup.value = false
  editingGroup.value = { ...group, tool_ids: [...(group.tool_ids || [])] }
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
  if (!editingGroup.value || editingGroup.value.tool_ids.length === 0) {
    alert(t('agent.selectToolsFirst'))
    return
  }

  aiGenerating.value = true
  generatedContent.value = ''
  
  const selectedTools = availableTools.value.filter(t => editingGroup.value?.tool_ids.includes(t.id))
  const toolsContext = selectedTools.map(t => `- ${t.name}: ${t.description}`).join('\n')
  
  const prompt = `Based on the following tools, please generate a name, description, and detailed instructions for an ability group.
Tools:
${toolsContext}

Return ONLY a JSON object with the following structure:
{
  "name": "Group Name",
  "description": "Short description",
  "instructions": "Detailed instructions on when and how to use this group"
}
Ensure the content is in the same language as the tool descriptions (or Chinese if unsure).
`

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

    await invoke('generate_plugin_stream', {
      request: {
        stream_id: streamId,
        message: prompt,
        system_prompt: "You are a helpful assistant that generates configuration for AI agent ability groups. Return valid JSON only.",
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
    'Network': '网络',
    'Security': '安全',
    'Data': '数据',
    'AI': 'AI',
    'System': '系统',
    'MCP': 'MCP',
    'Plugin': '插件',
    'Workflow': '工作流',
  }
  return nameMap[category] || category
}

const getCategoryBadgeClass = (category: string) => {
  const map: Record<string, string> = {
    'Network': 'badge-info',
    'Security': 'badge-error',
    'Data': 'badge-success',
    'AI': 'badge-warning',
    'System': 'badge-neutral',
    'MCP': 'badge-primary',
    'Plugin': 'badge-secondary',
    'Workflow': 'badge-accent',
  }
  return map[category] || 'badge-ghost'
}

onMounted(() => {
  loadGroups()
  loadTools()
})
</script>

<style scoped>
.ability-group-manager {
  max-height: 80vh;
  overflow-y: auto;
}
</style>

