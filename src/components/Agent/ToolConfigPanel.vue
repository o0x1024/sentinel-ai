<template>
  <div class="tool-config-panel">
    <!-- Header -->
    <div class="panel-header flex items-center justify-between p-4 border-b border-base-300">
      <div class="flex items-center gap-2">
        <i class="fas fa-tools text-primary"></i>
        <h3 class="text-lg font-semibold">{{ t('agent.toolConfig') }}</h3>
      </div>
      <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- Content -->
    <div class="panel-content p-4 space-y-4 overflow-y-auto">
      <!-- Enable Tools Toggle -->
      <div class="form-control">
        <label class="label cursor-pointer justify-start gap-3">
          <input 
            type="checkbox" 
            v-model="localConfig.enabled" 
            class="checkbox checkbox-primary"
            @change="emitUpdate"
          />
          <div>
            <span class="label-text font-medium">{{ t('agent.enableToolCalls') }}</span>
            <p class="text-xs text-base-content/60 mt-1">{{ t('agent.allowAgentToCallTools') }}</p>
          </div>
        </label>
      </div>

      <div v-if="localConfig.enabled" class="space-y-4">
        <!-- Tool Selection Strategy -->
        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">{{ t('agent.toolSelectionStrategy') }}</span>
          </label>
          <select 
            v-model="localConfig.selection_strategy" 
            class="select select-bordered w-full"
            @change="emitUpdate"
          >
            <option value="Keyword">{{ t('agent.keywordMatching') }}</option>
            <option value="LLM">{{ t('agent.intelligentAnalysis') }}</option>
            <option value="Hybrid">{{ t('agent.hybridStrategy') }}</option>
            <option value="Manual">{{ t('agent.manualSelection') }}</option>
            <option value="Ability">{{ t('agent.abilityMode') }}</option>
            <option value="All">{{ t('agent.allTools') }}</option>
          </select>
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ getStrategyDescription(localConfig.selection_strategy) }}
            </span>
          </label>
        </div>

        <!-- Max Tools -->
        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">{{ t('agent.maxTools') }}</span>
            <span class="label-text-alt">{{ localConfig.max_tools }}</span>
          </label>
          <input 
            type="range" 
            v-model.number="localConfig.max_tools" 
            min="1" 
            max="20" 
            class="range range-primary range-sm"
            @change="emitUpdate"
          />
          <div class="w-full flex justify-between text-xs px-2 text-base-content/60">
            <span>1</span>
            <span>5</span>
            <span>10</span>
            <span>15</span>
            <span>20</span>
          </div>
        </div>

        <!-- Ability Mode: Group Selection -->
        <div v-if="localConfig.selection_strategy === 'Ability'" class="form-control">
          <label class="label">
            <span class="label-text font-medium">{{ t('agent.abilityGroups') }}</span>
            <div class="flex gap-1">
              <button @click="loadAbilityGroups" class="btn btn-xs btn-ghost">
                <i class="fas fa-sync-alt"></i>
              </button>
              <button @click="showAbilityManager = true" class="btn btn-xs btn-primary btn-outline">
                <i class="fas fa-cog"></i>
                {{ t('agent.manage') }}
              </button>
            </div>
          </label>

          <div v-if="loadingAbilityGroups" class="flex justify-center py-4">
            <span class="loading loading-spinner loading-sm"></span>
          </div>

          <div v-else-if="abilityGroups.length === 0" class="alert alert-warning">
            <i class="fas fa-exclamation-triangle"></i>
            <span>{{ t('agent.noAbilityGroupsHint') }}</span>
          </div>

          <div v-else class="space-y-2 border border-base-300 rounded-lg p-3 max-h-48 overflow-y-auto">
            <p class="text-xs text-base-content/60 mb-2">{{ t('agent.selectAllowedGroups') }}</p>
            <label 
              v-for="group in abilityGroups" 
              :key="group.id"
              class="flex items-center gap-2 p-2 hover:bg-base-200 rounded cursor-pointer"
            >
              <input 
                type="checkbox"
                :value="group.id"
                v-model="localConfig.ability_groups"
                class="checkbox checkbox-sm checkbox-primary"
                @change="emitUpdate"
              />
              <div class="flex-1 min-w-0">
                <div class="font-medium text-sm">{{ group.name }}</div>
                <div class="text-xs text-base-content/60 truncate">{{ group.description }}</div>
              </div>
            </label>
          </div>

          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ localConfig.ability_groups?.length || 0 }} {{ t('agent.groupsSelected') }}（{{ t('agent.emptyMeansAll') }}）
            </span>
          </label>
        </div>

        <!-- Ability Group Manager Modal -->
        <dialog
          ref="abilityDialogRef"
          class="modal"
          @close="showAbilityManager = false"
        >
          <div class="modal-box max-w-2xl">
            <AbilityGroupManager 
              v-if="showAbilityManager"
              @close="showAbilityManager = false"
              @changed="loadAbilityGroups"
            />
          </div>
          <form method="dialog" class="modal-backdrop" @submit.prevent="showAbilityManager = false">
            <button>close</button>
          </form>
        </dialog>

        <!-- Tool Management (hidden in Ability mode) -->
        <div v-if="localConfig.selection_strategy !== 'Ability'" class="form-control">
          <label class="label">
            <span class="label-text font-medium">
              {{ localConfig.selection_strategy === 'Manual' ? t('agent.selectTools') : t('agent.toolManagement') }}
            </span>
            <button @click="loadTools" class="btn btn-xs btn-ghost">
              <i class="fas fa-sync-alt"></i>
            </button>
          </label>
          
          <div v-if="loading" class="flex justify-center py-4">
            <span class="loading loading-spinner loading-md"></span>
          </div>
          
          <div v-else class="space-y-2 max-h-96 overflow-y-auto border border-base-300 rounded-lg p-3">
            <!-- Search Box -->
            <div class="sticky top-0 z-10 bg-base-100 pb-2 -mt-1 pt-1">
              <div class="relative w-full">
                <input 
                  type="text" 
                  v-model="searchQuery"
                  :placeholder="t('agent.searchToolNamesOrDescriptions')" 
                  class="input input-sm input-bordered w-full pr-8" 
                />
                <div class="absolute inset-y-0 right-0 flex items-center pr-2">
                  <button 
                    v-if="searchQuery" 
                    @click="searchQuery = ''"
                    class="btn btn-ghost btn-xs btn-circle h-5 w-5 min-h-0"
                  >
                    <i class="fas fa-times text-xs"></i>
                  </button>
                  <i v-else class="fas fa-search text-xs text-base-content/50"></i>
                </div>
              </div>
            </div>

            <!-- Category Filters -->
            <div class="flex flex-wrap gap-2 mb-3 pb-3 border-b border-base-300">
              <!-- 全部按钮 -->
              <button 
                @click="clearCategoryFilter"
                class="btn btn-xs"
                :class="selectedCategories.length === 0 ? 'btn-primary' : 'btn-ghost'"
              >
                {{ t('agent.all') }}
              </button>
              
              <!-- 工具插件按钮 -->
              <button 
                v-if="hasPluginTools"
                @click="toggleCategory('Plugin')"
                class="btn btn-xs"
                :class="selectedCategories.includes('Plugin') ? 'btn-primary' : 'btn-ghost'"
              >
                {{ t('agent.plugins') }}
              </button>
              
              <!-- 其他分类按钮 -->
              <button 
                v-for="cat in categories.filter(c => c !== 'Plugin')" 
                :key="cat"
                @click="toggleCategory(cat)"
                class="btn btn-xs"
                :class="selectedCategories.includes(cat) ? 'btn-primary' : 'btn-ghost'"
              >
                {{ getCategoryDisplayName(cat) }}
              </button>
            </div>

            <!-- Tool List -->
            <div v-for="tool in filteredTools" :key="tool.id" class="form-control hover:bg-base-200 rounded px-2 transition-colors">
              <label v-if="localConfig.selection_strategy === 'Manual'" class="label cursor-pointer justify-start gap-3 py-2">
                <input 
                  type="checkbox" 
                  :value="tool.id"
                  v-model="localConfig.manual_tools"
                  class="checkbox checkbox-sm checkbox-primary"
                  @change="emitUpdate"
                />
                <div class="flex-1">
                  <div class="flex items-center gap-2">
                    <span class="font-medium text-sm">{{ tool.name }}</span>
                    <span class="badge badge-xs" :class="getCategoryBadgeClass(tool.category)">
                      {{ getCategoryDisplayName(tool.category) }}
                    </span>
                  </div>
                  <p class="text-xs text-base-content/60 mt-1 truncate" :title="tool.description">{{ tool.description }}</p>
                </div>
              </label>

              <div v-else class="flex items-center justify-between py-2">
                <div class="flex-1 min-w-0 mr-2">
                  <div class="flex items-center gap-2">
                    <span class="font-medium text-sm truncate">{{ tool.name }}</span>
                    <span class="badge badge-xs flex-shrink-0" :class="getCategoryBadgeClass(tool.category)">
                      {{ getCategoryDisplayName(tool.category) }}
                    </span>
                  </div>
                  <p class="text-xs text-base-content/60 mt-1 truncate" :title="tool.description">{{ tool.description }}</p>
                </div>
                <div class="join flex-shrink-0">
                   <button 
                     class="join-item btn btn-xs" 
                     :class="!isFixed(tool.id) && !isDisabled(tool.id) ? 'btn-active shadow-inner' : 'btn-ghost'"
                     @click="setToolStatus(tool.id, 'auto')"
                     :title="t('agent.autoSelect')"
                   >{{ t('agent.autoSelect') }}</button>
                   <button 
                     class="join-item btn btn-xs"
                     :class="isFixed(tool.id) ? 'btn-primary' : 'btn-ghost'"
                     @click="setToolStatus(tool.id, 'fixed')"
                     :title="t('agent.alwaysEnabled')"
                   >{{ t('agent.alwaysEnabled') }}</button>
                   <button 
                     class="join-item btn btn-xs"
                     :class="isDisabled(tool.id) ? 'btn-error' : 'btn-ghost'"
                     @click="setToolStatus(tool.id, 'disabled')"
                     :title="t('agent.disableTool')"
                   >{{ t('agent.disableTool') }}</button>
                </div>
              </div>
            </div>

            <div v-if="filteredTools.length === 0" class="text-center py-4 text-base-content/60">
              <i class="fas fa-inbox text-2xl mb-2"></i>
              <p class="text-sm">{{ t('agent.noToolsFound') }}</p>
            </div>
          </div>
        </div>

        <!-- Fixed Tools -->
        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">{{ t('agent.alwaysEnabledTools') }}</span>
          </label>
          <div class="flex flex-wrap gap-2">
            <div 
              v-for="tool in localConfig.fixed_tools" 
              :key="tool"
              class="badge badge-primary gap-2"
            >
              {{ tool }}
              <button 
                @click="removeFixedTool(tool)"
                class="btn btn-xs btn-ghost btn-circle"
              >
                <i class="fas fa-times text-xs"></i>
              </button>
            </div>
            <button 
              v-if="localConfig.fixed_tools.length === 0"
              class="badge badge-ghost"
            >
              {{ t('agent.none') }}
            </button>
          </div>
        </div>

        <!-- Tool Statistics -->
        <div v-if="statistics" class="stats stats-vertical shadow w-full">
          <div class="stat">
            <div class="stat-title">{{ t('agent.totalAvailableTools') }}</div>
            <div class="stat-value text-primary">{{ statistics.total_tools }}</div>
            <div class="stat-desc">
              {{ t('agent.builtin') }}: {{ statistics.builtin_tools }} | 
              {{ t('agent.workflow') }}: {{ statistics.workflow_tools }} | 
              MCP: {{ statistics.mcp_tools }} | 
              {{ t('agent.plugins') }}: {{ statistics.plugin_tools }}
            </div>
          </div>
        </div>

        <!-- Tool Usage Statistics -->
        <div class="divider">{{ t('agent.usageStatistics') }}</div>
        
        <div v-if="usageStats" class="space-y-3">
          <div class="stats stats-horizontal shadow w-full">
            <div class="stat">
              <div class="stat-title">{{ t('agent.totalExecutions') }}</div>
              <div class="stat-value text-sm">{{ usageStats.total_executions }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ t('agent.success') }}</div>
              <div class="stat-value text-sm text-success">{{ usageStats.successful_executions }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ t('agent.failure') }}</div>
              <div class="stat-value text-sm text-error">{{ usageStats.failed_executions }}</div>
            </div>
          </div>

          <!-- Top Used Tools -->
          <div v-if="topUsedTools.length > 0" class="space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-sm font-medium">{{ t('agent.mostUsedTools') }}</span>
              <button @click="loadUsageStats" class="btn btn-xs btn-ghost">
                <i class="fas fa-sync-alt"></i>
              </button>
            </div>
            <div class="space-y-1">
              <div 
                v-for="tool in topUsedTools.slice(0, 5)" 
                :key="tool.tool_id"
                class="flex items-center justify-between p-2 bg-base-200 rounded text-xs"
              >
                <div class="flex-1">
                  <div class="font-medium">{{ tool.tool_name }}</div>
                  <div class="text-base-content/60">
                    {{ t('agent.successRate') }}: {{ ((tool.success_count / tool.execution_count) * 100).toFixed(1) }}% | 
                    {{ t('agent.averageTime') }}: {{ tool.avg_execution_time_ms.toFixed(0) }}ms
                  </div>
                </div>
                <div class="badge badge-sm">{{ tool.execution_count }}{{ t('agent.times') }}</div>
              </div>
            </div>
          </div>

          <!-- Recent Executions -->
          <div v-if="usageStats.recent_executions.length > 0" class="space-y-2">
            <div class="text-sm font-medium">{{ t('agent.recentExecutions') }}</div>
            <div class="space-y-1 max-h-48 overflow-y-auto">
              <div 
                v-for="record in usageStats.recent_executions.slice(0, 10)" 
                :key="`${record.execution_id}-${record.timestamp}`"
                class="flex items-center gap-2 p-2 bg-base-200 rounded text-xs"
              >
                <i 
                  class="fas" 
                  :class="record.success ? 'fa-check-circle text-success' : 'fa-times-circle text-error'"
                ></i>
                <div class="flex-1">
                  <div class="font-medium">{{ record.tool_name }}</div>
                  <div class="text-base-content/60">
                    {{ formatTimestamp(record.timestamp) }} | {{ record.execution_time_ms }}ms
                  </div>
                </div>
              </div>
            </div>
          </div>

          <button @click="clearUsageStats" class="btn btn-sm btn-error btn-outline w-full">
            <i class="fas fa-trash"></i>
            {{ t('agent.clearStatistics') }}
          </button>
        </div>

        <div v-else class="text-center py-4 text-base-content/60">
          <i class="fas fa-chart-bar text-2xl mb-2"></i>
          <p class="text-sm">{{ t('agent.noUsageStatistics') }}</p>
        </div>
      </div>
    </div>

    <!-- Footer Actions -->
    <div class="panel-footer p-4 border-t border-base-300 flex justify-end gap-2">
      <button @click="resetToDefault" class="btn btn-sm btn-ghost">
        <i class="fas fa-undo"></i>
        {{ t('agent.reset') }}
      </button>
      <button @click="$emit('close')" class="btn btn-sm btn-primary">
        <i class="fas fa-check"></i>
        {{ t('agent.confirm') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import AbilityGroupManager from './AbilityGroupManager.vue'

interface ToolMetadata {
  id: string
  name: string
  description: string
  category: string
  tags: string[]
  cost_estimate: string
  always_available: boolean
}

interface AbilityGroupSummary {
  id: string
  name: string
  description: string
}

interface ToolConfig {
  enabled: boolean
  selection_strategy: string
  max_tools: number
  fixed_tools: string[]
  disabled_tools: string[]
  manual_tools?: string[]
  ability_groups?: string[]
}

interface ToolStatistics {
  total_tools: number
  builtin_tools: number
  workflow_tools: number
  mcp_tools: number
  plugin_tools: number
  always_available: number
  by_category: Record<string, number>
  by_cost: Record<string, number>
}

interface ToolUsageStats {
  tool_id: string
  tool_name: string
  execution_count: number
  success_count: number
  failure_count: number
  avg_execution_time_ms: number
  last_used: number
}

interface ToolUsageRecord {
  tool_id: string
  tool_name: string
  execution_id: string
  timestamp: number
  success: boolean
  execution_time_ms: number
  error_message?: string
}

interface ToolUsageStatistics {
  total_executions: number
  successful_executions: number
  failed_executions: number
  by_tool: Record<string, ToolUsageStats>
  recent_executions: ToolUsageRecord[]
}

const props = defineProps<{
  config: ToolConfig
}>()

const emit = defineEmits<{
  'update:config': [config: ToolConfig]
  'close': []
}>()

// 初始化 localConfig，处理 Manual/Ability 枚举格式
const initLocalConfig = () => {
  let manualTools: string[] = []
  let abilityGroupIds: string[] = []
  const rawStrategy = props.config.selection_strategy
  let strategy: string = 'Keyword'
  
  if (rawStrategy !== null && rawStrategy !== undefined) {
    // 如果 selection_strategy 是枚举格式 { Manual: [...] } 或 { Ability: [...] }，提取列表
    if (typeof rawStrategy === 'object') {
      const manualValue = (rawStrategy as any).Manual
      const abilityValue = (rawStrategy as any).Ability
      if (manualValue) {
        // 统一工具 ID 格式：将 :: 替换为 __
        manualTools = manualValue.map((id: string) => id.replace(/::/g, '__'))
        strategy = 'Manual'
      } else if (abilityValue !== undefined) {
        abilityGroupIds = abilityValue || []
        strategy = 'Ability'
      }
    } else {
      strategy = rawStrategy as string
    }
  }
  
  return { 
    ...props.config, 
    selection_strategy: strategy,
    manual_tools: manualTools,
    ability_groups: abilityGroupIds,
  }
}

const { t } = useI18n()

const localConfig = ref<ToolConfig>(initLocalConfig())
const allTools = ref<ToolMetadata[]>([])
const statistics = ref<ToolStatistics | null>(null)
const usageStats = ref<ToolUsageStatistics | null>(null)
const loading = ref(false)
const selectedCategories = ref<string[]>([])
const searchQuery = ref('')

// Ability mode state
const abilityGroups = ref<AbilityGroupSummary[]>([])
const loadingAbilityGroups = ref(false)
const showAbilityManager = ref(false)
const abilityDialogRef = ref<HTMLDialogElement | null>(null)

watch(showAbilityManager, async open => {
  // Use native dialog API for stability across webviews
  await nextTick()
  const el = abilityDialogRef.value
  if (!el) return
  if (open) {
    if (!el.open) el.showModal()
  } else {
    if (el.open) el.close()
  }
})

const categories = computed(() => {
  const cats = new Set(allTools.value.map(t => t.category))
  return Array.from(cats).sort()
})

const hasPluginTools = computed(() => {
  return allTools.value.some(t => t.category === 'Plugin')
})

const filteredTools = computed(() => {
  let tools = allTools.value

  // Filter by search query
  if (searchQuery.value.trim()) {
    const query = searchQuery.value.toLowerCase()
    tools = tools.filter(t => 
      t.name.toLowerCase().includes(query) || 
      t.description.toLowerCase().includes(query)
    )
  }

  // Filter by category
  if (selectedCategories.value.length === 0) {
    return tools
  }
  return tools.filter(t => selectedCategories.value.includes(t.category))
})

const topUsedTools = computed(() => {
  if (!usageStats.value) return []
  return Object.values(usageStats.value.by_tool)
    .sort((a, b) => b.execution_count - a.execution_count)
})

const toggleCategory = (cat: string) => {
  const index = selectedCategories.value.indexOf(cat)
  if (index > -1) {
    selectedCategories.value.splice(index, 1)
  } else {
    selectedCategories.value.push(cat)
  }
}

const clearCategoryFilter = () => {
  selectedCategories.value = []
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

const getStrategyDescription = (strategy: string) => {
  const descriptions: Record<string, string> = {
    'Keyword': '基于关键词匹配，速度快，无额外成本',
    'LLM': '使用 LLM 智能分析任务，准确度高，有少量 token 成本',
    'Hybrid': '关键词初筛 + LLM 精选，兼顾速度和准确度',
    'Manual': '手动选择需要的工具',
    'Ability': '渐进式披露：先选工具组，再暴露组内工具，token 可控',
    'All': '使用所有可用工具（不推荐，token 消耗大）',
  }
  return descriptions[strategy] || ''
}

const loadTools = async () => {
  loading.value = true
  try {
    allTools.value = await invoke<ToolMetadata[]>('get_all_tool_metadata')
    statistics.value = await invoke<ToolStatistics>('get_tool_statistics')
  } catch (error) {
    console.error('Failed to load tools:', error)
  } finally {
    loading.value = false
  }
}

const loadAbilityGroups = async () => {
  loadingAbilityGroups.value = true
  try {
    abilityGroups.value = await invoke<AbilityGroupSummary[]>('list_ability_groups')
  } catch (error) {
    console.error('Failed to load ability groups:', error)
  } finally {
    loadingAbilityGroups.value = false
  }
}

const loadUsageStats = async () => {
  try {
    usageStats.value = await invoke<ToolUsageStatistics>('get_tool_usage_stats')
  } catch (error) {
    console.error('Failed to load usage stats:', error)
  }
}

const clearUsageStats = async () => {
  if (!confirm(t('agent.areYouSureClearStatistics'))) return
  
  try {
    await invoke('clear_tool_usage_stats')
    usageStats.value = null
    await loadUsageStats()
  } catch (error) {
    console.error('Failed to clear usage stats:', error)
  }
}

const formatTimestamp = (timestamp: number) => {
  const date = new Date(timestamp * 1000)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  
  if (diff < 60000) return '刚刚'
  if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}小时前`
  return date.toLocaleDateString()
}

const isFixed = (id: string) => localConfig.value.fixed_tools?.includes(id)
const isDisabled = (id: string) => localConfig.value.disabled_tools?.includes(id)

const setToolStatus = (id: string, status: 'auto' | 'fixed' | 'disabled') => {
  if (!localConfig.value.fixed_tools) localConfig.value.fixed_tools = []
  if (!localConfig.value.disabled_tools) localConfig.value.disabled_tools = []
  
  // Remove from both
  localConfig.value.fixed_tools = localConfig.value.fixed_tools.filter(t => t !== id)
  localConfig.value.disabled_tools = localConfig.value.disabled_tools.filter(t => t !== id)
  
  if (status === 'fixed') {
    localConfig.value.fixed_tools.push(id)
  } else if (status === 'disabled') {
    localConfig.value.disabled_tools.push(id)
  }
  
  emitUpdate()
}

const removeFixedTool = (tool: string) => {
  const index = localConfig.value.fixed_tools.indexOf(tool)
  if (index > -1) {
    localConfig.value.fixed_tools.splice(index, 1)
    emitUpdate()
  }
}

const resetToDefault = () => {
  localConfig.value = {
    enabled: false,
    selection_strategy: 'Keyword',
    max_tools: 5,
    fixed_tools: ['local_time'],
    disabled_tools: [],
    manual_tools: [],
    ability_groups: [],
  }
  emitUpdate()
}

const emitUpdate = () => {
  // 处理枚举格式的策略转换
  const configToEmit = { ...localConfig.value }
  
  if (configToEmit.selection_strategy === 'Manual' && configToEmit.manual_tools) {
    // 统一工具 ID 格式：将 :: 替换为 __，并去重
    const normalizedTools = [...new Set(
      configToEmit.manual_tools.map((id: string) => id.replace(/::/g, '__'))
    )]
    console.log('[ToolConfigPanel] Emitting manual_tools:', normalizedTools)
    // 将 selection_strategy 转换为 Rust 枚举格式: { Manual: [...] }
    configToEmit.selection_strategy = { Manual: normalizedTools } as any
    delete configToEmit.manual_tools
  } else if (configToEmit.selection_strategy === 'Ability') {
    // 将 selection_strategy 转换为 Rust 枚举格式: { Ability: [...] }
    const groupIds = configToEmit.ability_groups || []
    console.log('[ToolConfigPanel] Emitting ability_groups:', groupIds)
    configToEmit.selection_strategy = { Ability: groupIds } as any
    delete configToEmit.ability_groups
  }
  
  emit('update:config', configToEmit)
}

watch(() => props.config, (newConfig) => {
  // 处理 Manual/Ability 枚举格式
  let manualTools: string[] = []
  let abilityGroupIds: string[] = []
  const rawStrategy = newConfig.selection_strategy
  let strategy: string = 'Keyword'
  
  if (rawStrategy !== null && rawStrategy !== undefined) {
    if (typeof rawStrategy === 'object') {
      const manualValue = (rawStrategy as any).Manual
      const abilityValue = (rawStrategy as any).Ability
      if (manualValue) {
        manualTools = manualValue.map((id: string) => id.replace(/::/g, '__'))
        strategy = 'Manual'
      } else if (abilityValue !== undefined) {
        abilityGroupIds = abilityValue || []
        strategy = 'Ability'
      }
    } else {
      strategy = rawStrategy as string
    }
  }
  
  localConfig.value = { 
    ...newConfig, 
    selection_strategy: strategy,
    manual_tools: manualTools,
    ability_groups: abilityGroupIds,
  }
}, { deep: true })

onMounted(() => {
  loadTools()
  loadUsageStats()
  loadAbilityGroups()
})
</script>

<style scoped>
.tool-config-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--fallback-b1, oklch(var(--b1)));
}

.panel-content {
  flex: 1;
  overflow-y: auto;
}

.icon-btn {
  @apply btn btn-sm btn-ghost btn-circle;
}

.icon-btn.active {
  @apply btn-primary;
}
</style>
