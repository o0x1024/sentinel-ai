<template>
  <AgentToolPolicyPanel
    title="工具绑定"
    description="为当前后台 Agent 选择 `required / optional / forbidden` 工具，用来约束事件驱动执行时的工具边界。"
  >
    <template #actions>
      <button class="btn btn-xs btn-ghost" @click="loadTools">
        <i class="fas fa-rotate mr-1"></i>
        刷新工具
      </button>
    </template>

    <template #summary>
      <span class="badge badge-primary badge-sm">{{ `Required ${localValue.requiredTools.length}` }}</span>
      <span class="badge badge-info badge-sm">{{ `Optional ${localValue.optionalTools.length}` }}</span>
      <span class="badge badge-error badge-sm">{{ `Forbidden ${localValue.forbiddenTools.length}` }}</span>
    </template>

    <div class="space-y-4">
      <div class="rounded-lg border border-info/30 bg-info/10 px-3 py-2 text-xs text-base-content/70">
        `sops` 默认会注入到后台型 Agent，用于读取当前 Profile 的 SOP 目录。通常不需要显式设为 `required`，只有在你要禁用它时才建议标记为 `forbidden`。
      </div>

      <div class="stats stats-horizontal shadow-sm w-full">
        <div class="stat px-4 py-3">
          <div class="stat-title text-xs">Required</div>
          <div class="stat-value text-primary text-lg">{{ localValue.requiredTools.length }}</div>
        </div>
        <div class="stat px-4 py-3">
          <div class="stat-title text-xs">Optional</div>
          <div class="stat-value text-info text-lg">{{ localValue.optionalTools.length }}</div>
        </div>
        <div class="stat px-4 py-3">
          <div class="stat-title text-xs">Forbidden</div>
          <div class="stat-value text-error text-lg">{{ localValue.forbiddenTools.length }}</div>
        </div>
      </div>

      <div class="flex flex-wrap gap-2">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索工具名称或描述"
          class="input input-sm input-bordered flex-1 min-w-[220px]"
        />
        <button
          class="btn btn-xs"
          :class="selectedCategories.length === 0 ? 'btn-primary' : 'btn-ghost'"
          @click="selectedCategories = []"
        >
          全部
        </button>
        <button
          v-for="category in allCategories"
          :key="category"
          class="btn btn-xs"
          :class="
            selectedCategories.includes(category) ? getCategoryBadgeClass(category) : 'btn-ghost'
          "
          @click="toggleCategory(category)"
        >
          <i :class="getCategoryIcon(category)" class="mr-1"></i>
          {{ getCategoryDisplayName(category) }}
        </button>
      </div>

      <div v-if="loading" class="py-10 text-center">
        <span class="loading loading-spinner loading-md"></span>
      </div>

      <div v-else class="border border-base-300 rounded-lg max-h-[420px] overflow-y-auto">
        <div
          v-for="tool in filteredTools"
          :key="tool.id"
          class="p-3 border-b border-base-300 last:border-b-0 hover:bg-base-200/60 transition-colors"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-medium text-sm truncate">{{ tool.name }}</span>
                <span v-if="tool.virtual" class="badge badge-xs badge-outline"> System </span>
                <span class="badge badge-xs" :class="getCategoryBadgeClass(tool.category)">
                  {{ getCategoryDisplayName(tool.category) }}
                </span>
                <span
                  v-if="currentStatus(tool.id) !== 'none'"
                  class="badge badge-xs"
                  :class="getStatusBadgeClass(currentStatus(tool.id))"
                >
                  {{ currentStatus(tool.id) }}
                </span>
              </div>
              <p class="text-xs text-base-content/60 mt-1 break-words">{{ tool.description }}</p>
            </div>

            <div class="join shrink-0">
              <button
                class="join-item btn btn-xs"
                :class="currentStatus(tool.id) === 'none' ? 'btn-active' : 'btn-ghost'"
                @click="setToolStatus(tool.id, 'none')"
              >
                默认
              </button>
              <button
                class="join-item btn btn-xs"
                :class="currentStatus(tool.id) === 'required' ? 'btn-primary' : 'btn-ghost'"
                @click="setToolStatus(tool.id, 'required')"
              >
                Required
              </button>
              <button
                class="join-item btn btn-xs"
                :class="currentStatus(tool.id) === 'optional' ? 'btn-info' : 'btn-ghost'"
                @click="setToolStatus(tool.id, 'optional')"
              >
                Optional
              </button>
              <button
                class="join-item btn btn-xs"
                :class="currentStatus(tool.id) === 'forbidden' ? 'btn-error' : 'btn-ghost'"
                @click="setToolStatus(tool.id, 'forbidden')"
              >
                Forbidden
              </button>
            </div>
          </div>
        </div>

        <div
          v-if="filteredTools.length === 0"
          class="py-10 text-center text-sm text-base-content/60"
        >
          没有匹配的工具
        </div>
      </div>
    </div>
  </AgentToolPolicyPanel>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AgentToolPolicyPanel from './AgentToolPolicyPanel.vue'
import {
  SYSTEM_AGENT_VIRTUAL_TOOLS,
  type SystemAgentToolMetadata as ToolMetadata,
} from './systemAgentSettingsSupport'

interface ToolBindingValue {
  requiredTools: string[]
  optionalTools: string[]
  forbiddenTools: string[]
}

const props = defineProps<{
  modelValue: ToolBindingValue
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ToolBindingValue]
}>()

const loading = ref(false)
const allTools = ref<ToolMetadata[]>([])
const searchQuery = ref('')
const selectedCategories = ref<string[]>([])
const localValue = ref<ToolBindingValue>({
  requiredTools: [],
  optionalTools: [],
  forbiddenTools: [],
})

const syncFromProps = () => {
  localValue.value = {
    requiredTools: [...(props.modelValue.requiredTools || [])],
    optionalTools: [...(props.modelValue.optionalTools || [])],
    forbiddenTools: [...(props.modelValue.forbiddenTools || [])],
  }
}

const allCategories = computed(() => {
  const categories = new Set(allTools.value.map(tool => tool.category))
  return Array.from(categories).sort()
})

const filteredTools = computed(() => {
  let tools = [...allTools.value].sort((left, right) => left.name.localeCompare(right.name))
  if (searchQuery.value.trim()) {
    const keyword = searchQuery.value.trim().toLowerCase()
    tools = tools.filter(
      tool =>
        tool.name.toLowerCase().includes(keyword) ||
        tool.description.toLowerCase().includes(keyword)
    )
  }
  if (selectedCategories.value.length > 0) {
    tools = tools.filter(tool => selectedCategories.value.includes(tool.category))
  }
  return tools
})

const loadTools = async () => {
  loading.value = true
  try {
    const runtimeTools = await invoke<ToolMetadata[]>('get_all_tool_metadata')
    const mergedTools = new Map<string, ToolMetadata>()
    for (const tool of SYSTEM_AGENT_VIRTUAL_TOOLS) {
      mergedTools.set(tool.id, tool)
    }
    for (const tool of runtimeTools) {
      mergedTools.set(tool.id, {
        ...tool,
        virtual: false,
      })
    }
    allTools.value = Array.from(mergedTools.values())
  } catch (error) {
    console.error('Failed to load system-agent tool metadata', error)
    allTools.value = [...SYSTEM_AGENT_VIRTUAL_TOOLS]
  } finally {
    loading.value = false
  }
}

const emitUpdate = () => {
  emit('update:modelValue', {
    requiredTools: [...localValue.value.requiredTools],
    optionalTools: [...localValue.value.optionalTools],
    forbiddenTools: [...localValue.value.forbiddenTools],
  })
}

const currentStatus = (toolId: string): 'none' | 'required' | 'optional' | 'forbidden' => {
  if (localValue.value.requiredTools.includes(toolId)) return 'required'
  if (localValue.value.forbiddenTools.includes(toolId)) return 'forbidden'
  if (localValue.value.optionalTools.includes(toolId)) return 'optional'
  return 'none'
}

const setToolStatus = (toolId: string, status: 'none' | 'required' | 'optional' | 'forbidden') => {
  localValue.value.requiredTools = localValue.value.requiredTools.filter(id => id !== toolId)
  localValue.value.optionalTools = localValue.value.optionalTools.filter(id => id !== toolId)
  localValue.value.forbiddenTools = localValue.value.forbiddenTools.filter(id => id !== toolId)

  if (status === 'required') {
    localValue.value.requiredTools.push(toolId)
  } else if (status === 'optional') {
    localValue.value.optionalTools.push(toolId)
  } else if (status === 'forbidden') {
    localValue.value.forbiddenTools.push(toolId)
  }

  emitUpdate()
}

const toggleCategory = (category: string) => {
  const index = selectedCategories.value.indexOf(category)
  if (index >= 0) {
    selectedCategories.value.splice(index, 1)
  } else {
    selectedCategories.value.push(category)
  }
}

const getCategoryDisplayName = (category: string) => {
  const nameMap: Record<string, string> = {
    network: '网络',
    security: '安全',
    data: '数据',
    ai: 'AI',
    system: '系统',
    mcp: 'MCP',
    plugin: '插件',
    workflow: '工作流',
    browser: '浏览器',
  }
  return nameMap[category.toLowerCase()] || category
}

const getCategoryBadgeClass = (category: string) => {
  const map: Record<string, string> = {
    network: 'badge-info',
    security: 'badge-error',
    data: 'badge-success',
    ai: 'badge-warning',
    system: 'badge-neutral',
    mcp: 'badge-primary',
    plugin: 'badge-secondary',
    workflow: 'badge-accent',
    browser: 'badge-primary',
  }
  return map[category.toLowerCase()] || 'badge-ghost'
}

const getCategoryIcon = (category: string) => {
  const map: Record<string, string> = {
    network: 'fas fa-network-wired',
    security: 'fas fa-shield-alt',
    data: 'fas fa-database',
    ai: 'fas fa-brain',
    system: 'fas fa-cog',
    mcp: 'fas fa-plug',
    plugin: 'fas fa-puzzle-piece',
    workflow: 'fas fa-project-diagram',
    browser: 'fas fa-globe',
  }
  return map[category.toLowerCase()] || 'fas fa-tools'
}

const getStatusBadgeClass = (status: string) => {
  if (status === 'required') return 'badge-primary'
  if (status === 'optional') return 'badge-info'
  if (status === 'forbidden') return 'badge-error'
  return 'badge-ghost'
}

watch(() => props.modelValue, syncFromProps, { deep: true, immediate: true })

onMounted(() => {
  loadTools()
})
</script>
