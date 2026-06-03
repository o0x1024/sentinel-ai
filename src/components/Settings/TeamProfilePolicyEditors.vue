<template>
  <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
    <section class="policy-card xl:col-span-2">
      <PolicyHeader
        title="工具角色矩阵"
        :mode="modes.toolPolicyMatrix"
        @update:mode="setMode('toolPolicyMatrix', $event)"
      />

      <div v-if="modes.toolPolicyMatrix === 'form'" class="grid grid-cols-1 gap-3">
        <div class="rounded-lg border border-info/30 bg-info/5 px-3 py-2 text-xs leading-5 text-base-content/70">
          Team 默认不再预选成员 Profile 工具。成员 Profile 没有显式工具范围时，运行时直接按这里的角色矩阵生效；如果成员 Profile 自己配置了工具范围，最终可用工具会再与这里取交集。
        </div>
        <div
          v-for="role in toolRoles"
          :key="role.key"
          class="rounded-lg border border-base-300 bg-base-200/30 p-3"
        >
          <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div>
              <div class="text-sm font-semibold">{{ role.label }}</div>
              <div class="text-xs text-base-content/60">{{ role.description }}</div>
            </div>
            <span class="badge badge-outline">{{ readRoleTools(role.key).length }} 个工具</span>
          </div>
          <ToolPolicyListPicker
            title="可用工具"
            placeholder="选择该角色在 Team 运行时允许使用的工具"
            :tools="allTools"
            :loading="loadingTools"
            :value="readRoleTools(role.key)"
            @reload="loadTools"
            @update:value="updateToolRoleTools(role.key, $event)"
          />
        </div>
      </div>
      <JsonPolicyTextarea
        v-else
        :draft="jsonDrafts.toolPolicyMatrix"
        :error="jsonErrors.toolPolicyMatrix"
        @input="updateJson('toolPolicyMatrix', $event)"
      />
    </section>

    <section class="policy-card">
      <PolicyHeader
        title="记忆策略"
        :mode="modes.memoryPolicy"
        @update:mode="setMode('memoryPolicy', $event)"
      />
      <div v-if="modes.memoryPolicy === 'form'" class="grid grid-cols-1 gap-3">
        <label class="form-control">
          <span class="label-text mb-1">写入门禁</span>
          <select
            class="select select-bordered select-sm"
            :value="memoryPolicy.monitorGate || 'candidate_then_orchestrator_accept'"
            @change="updatePolicyField('memoryPolicy', 'monitorGate', ($event.target as HTMLSelectElement).value)"
          >
            <option value="candidate_then_orchestrator_accept">Monitor 候选，Orchestrator 确认</option>
            <option value="monitor_only">Monitor 直接接受</option>
            <option value="orchestrator_only">仅 Orchestrator 写入</option>
          </select>
        </label>
        <label class="form-control">
          <span class="label-text mb-1">共享范围</span>
          <select
            class="select select-bordered select-sm"
            :value="memoryPolicy.shareScope || 'high_value_only'"
            @change="updatePolicyField('memoryPolicy', 'shareScope', ($event.target as HTMLSelectElement).value)"
          >
            <option value="high_value_only">仅高价值信息</option>
            <option value="evidence_risk_decision">证据 / 风险 / 决策</option>
            <option value="evidence_risk_blocker_checkpoint">证据 / 风险 / 阻塞 / Checkpoint</option>
          </select>
        </label>
        <label class="label cursor-pointer justify-start gap-3 rounded-lg border border-base-300 bg-base-200/30 px-3">
          <input
            type="checkbox"
            class="checkbox checkbox-primary checkbox-sm"
            :checked="memoryPolicy.longTermMemory === true"
            @change="updatePolicyField('memoryPolicy', 'longTermMemory', ($event.target as HTMLInputElement).checked)"
          />
          <span class="label-text">允许高价值记忆进入长期记忆</span>
        </label>
      </div>
      <JsonPolicyTextarea
        v-else
        :draft="jsonDrafts.memoryPolicy"
        :error="jsonErrors.memoryPolicy"
        @input="updateJson('memoryPolicy', $event)"
      />
    </section>

    <section class="policy-card">
      <PolicyHeader
        title="Harness 策略"
        :mode="modes.harnessPolicy"
        @update:mode="setMode('harnessPolicy', $event)"
      />
      <div v-if="modes.harnessPolicy === 'form'" class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <NumberField
          label="心跳间隔（秒）"
          :value="Number(harnessPolicy.heartbeatSecs || 30)"
          @update:value="updatePolicyField('harnessPolicy', 'heartbeatSecs', $event)"
        />
        <NumberField
          label="租约时长（秒）"
          :value="Number(harnessPolicy.leaseSecs || 600)"
          @update:value="updatePolicyField('harnessPolicy', 'leaseSecs', $event)"
        />
        <label class="form-control">
          <span class="label-text mb-1">Checkpoint 类型</span>
          <select
            class="select select-bordered select-sm"
            :value="harnessPolicy.checkpoint || 'event_sequence'"
            @change="updatePolicyField('harnessPolicy', 'checkpoint', ($event.target as HTMLSelectElement).value)"
          >
            <option value="event_sequence">事件序列</option>
            <option value="task_boundary">任务边界</option>
            <option value="manual">手动</option>
          </select>
        </label>
        <label class="label cursor-pointer justify-start gap-3 rounded-lg border border-base-300 bg-base-200/30 px-3">
          <input
            type="checkbox"
            class="checkbox checkbox-primary checkbox-sm"
            :checked="harnessPolicy.allowResume === true"
            @change="updatePolicyField('harnessPolicy', 'allowResume', ($event.target as HTMLInputElement).checked)"
          />
          <span class="label-text">允许恢复长任务</span>
        </label>
      </div>
      <JsonPolicyTextarea
        v-else
        :draft="jsonDrafts.harnessPolicy"
        :error="jsonErrors.harnessPolicy"
        @input="updateJson('harnessPolicy', $event)"
      />
    </section>

    <section class="policy-card">
      <PolicyHeader
        title="并发策略"
        :mode="modes.concurrencyPolicy"
        @update:mode="setMode('concurrencyPolicy', $event)"
      />
      <div v-if="modes.concurrencyPolicy === 'form'" class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <NumberField
          label="最大并发 Specialist"
          :value="Number(concurrencyPolicy.maxSpecialists || 1)"
          @update:value="updatePolicyField('concurrencyPolicy', 'maxSpecialists', $event)"
        />
        <NumberField
          label="每 Specialist 最大任务数"
          :value="Number(concurrencyPolicy.maxTasksPerSpecialist || 1)"
          @update:value="updatePolicyField('concurrencyPolicy', 'maxTasksPerSpecialist', $event)"
        />
      </div>
      <JsonPolicyTextarea
        v-else
        :draft="jsonDrafts.concurrencyPolicy"
        :error="jsonErrors.concurrencyPolicy"
        @input="updateJson('concurrencyPolicy', $event)"
      />
    </section>

    <section class="policy-card">
      <PolicyHeader
        title="安全策略"
        :mode="modes.safetyPolicy"
        @update:mode="setMode('safetyPolicy', $event)"
      />
      <div v-if="modes.safetyPolicy === 'form'" class="grid grid-cols-1 gap-2">
        <BooleanPolicySwitch
          label="Orchestrator 禁止危险工具"
          :checked="safetyPolicy.orchestratorNoDangerousTools === true"
          @update:checked="updatePolicyField('safetyPolicy', 'orchestratorNoDangerousTools', $event)"
        />
        <BooleanPolicySwitch
          label="Monitor 只读"
          :checked="safetyPolicy.monitorReadOnly === true"
          @update:checked="updatePolicyField('safetyPolicy', 'monitorReadOnly', $event)"
        />
        <BooleanPolicySwitch
          label="高风险工具需要审批"
          :checked="safetyPolicy.requireApprovalForHighRiskTools === true"
          @update:checked="updatePolicyField('safetyPolicy', 'requireApprovalForHighRiskTools', $event)"
        />
        <BooleanPolicySwitch
          label="失败时等待已启动任务后停止"
          :checked="safetyPolicy.failFastWaitStartedAssignments === true"
          @update:checked="updatePolicyField('safetyPolicy', 'failFastWaitStartedAssignments', $event)"
        />
      </div>
      <JsonPolicyTextarea
        v-else
        :draft="jsonDrafts.safetyPolicy"
        :error="jsonErrors.safetyPolicy"
        @input="updateJson('safetyPolicy', $event)"
      />
    </section>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, defineComponent, h, onMounted, reactive, ref, watch } from 'vue'
import type { PropType } from 'vue'

type PolicyKey = 'toolPolicyMatrix' | 'memoryPolicy' | 'harnessPolicy' | 'concurrencyPolicy' | 'safetyPolicy'
type PolicyMode = 'form' | 'json'
type ToolRoleKey = 'orchestrator' | 'specialist' | 'monitor'

interface ToolMetadata {
  id: string
  name: string
  description: string
  category: string
  tags?: string[]
}

const props = defineProps<{
  toolPolicyMatrix: Record<string, any>
  memoryPolicy: Record<string, any>
  harnessPolicy: Record<string, any>
  concurrencyPolicy: Record<string, any>
  safetyPolicy: Record<string, any>
}>()

const emit = defineEmits<{
  'update:toolPolicyMatrix': [value: Record<string, any>]
  'update:memoryPolicy': [value: Record<string, any>]
  'update:harnessPolicy': [value: Record<string, any>]
  'update:concurrencyPolicy': [value: Record<string, any>]
  'update:safetyPolicy': [value: Record<string, any>]
}>()

const toolRoles: Array<{ key: ToolRoleKey; label: string; description: string }> = [
  { key: 'orchestrator', label: 'Orchestrator', description: '拆解、编排、恢复与汇总的工具白名单' },
  { key: 'specialist', label: 'Specialist', description: '执行具体领域子任务的工具白名单' },
  { key: 'monitor', label: 'Monitor', description: '指标采集、质量评估与重试建议的工具白名单' },
]

const modes = reactive<Record<PolicyKey, PolicyMode>>({
  toolPolicyMatrix: 'form',
  memoryPolicy: 'form',
  harnessPolicy: 'form',
  concurrencyPolicy: 'form',
  safetyPolicy: 'form',
})

const jsonDrafts = reactive<Record<PolicyKey, string>>({
  toolPolicyMatrix: '',
  memoryPolicy: '',
  harnessPolicy: '',
  concurrencyPolicy: '',
  safetyPolicy: '',
})

const jsonErrors = reactive<Record<PolicyKey, string>>({
  toolPolicyMatrix: '',
  memoryPolicy: '',
  harnessPolicy: '',
  concurrencyPolicy: '',
  safetyPolicy: '',
})

const allTools = ref<ToolMetadata[]>([])
const loadingTools = ref(false)

const memoryPolicy = computed(() => props.memoryPolicy || {})
const harnessPolicy = computed(() => props.harnessPolicy || {})
const concurrencyPolicy = computed(() => props.concurrencyPolicy || {})
const safetyPolicy = computed(() => props.safetyPolicy || {})

const policyValue = (key: PolicyKey) => props[key] || {}

const emitPolicy = (key: PolicyKey, value: Record<string, any>) => {
  emit(`update:${key}` as any, value)
  jsonDrafts[key] = JSON.stringify(value, null, 2)
  jsonErrors[key] = ''
}

const setMode = (key: PolicyKey, mode: PolicyMode) => {
  modes[key] = mode
  if (mode === 'json') {
    jsonDrafts[key] = JSON.stringify(policyValue(key), null, 2)
    jsonErrors[key] = ''
  }
}

const normalizeList = (value: unknown): string[] => {
  if (!Array.isArray(value)) return []
  const seen = new Set<string>()
  const out: string[] = []
  value.forEach((item) => {
    if (typeof item !== 'string') return
    const normalized = item.trim().replace(/::/g, '__')
    if (!normalized || seen.has(normalized)) return
    seen.add(normalized)
    out.push(normalized)
  })
  return out
}

const parseListText = (value: string) =>
  normalizeList(value.split(/[,\n]/).map(item => item.trim()))

const getToolRole = (role: ToolRoleKey): Record<string, any> => {
  const rolePolicy = props.toolPolicyMatrix?.[role]
  return rolePolicy && typeof rolePolicy === 'object' && !Array.isArray(rolePolicy)
    ? rolePolicy
    : {}
}

const readRoleTools = (role: ToolRoleKey) => normalizeList(getToolRole(role).tools)

const updateToolRole = (role: ToolRoleKey, updater: (current: Record<string, any>) => Record<string, any>) => {
  const currentMatrix = props.toolPolicyMatrix || {}
  emitPolicy('toolPolicyMatrix', {
    ...currentMatrix,
    [role]: updater(getToolRole(role)),
  })
}

const updateToolRoleTools = (role: ToolRoleKey, value: unknown) => {
  updateToolRole(role, () => ({
    tools: normalizeList(value),
  }))
}

const updatePolicyField = (key: Exclude<PolicyKey, 'toolPolicyMatrix'>, field: string, value: unknown) => {
  emitPolicy(key, {
    ...policyValue(key),
    [field]: value,
  })
}

const updateJson = (key: PolicyKey, value: string) => {
  jsonDrafts[key] = value
  try {
    const parsed = JSON.parse(value)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      jsonErrors[key] = 'JSON 必须是对象。'
      return
    }
    jsonErrors[key] = ''
    emit(`update:${key}` as any, parsed)
  } catch (error: any) {
    jsonErrors[key] = error?.message || 'JSON 解析失败。'
  }
}

const loadTools = async () => {
  loadingTools.value = true
  try {
    allTools.value = await invoke<ToolMetadata[]>('get_all_tool_metadata')
  } catch (error) {
    console.error('[TeamProfilePolicyEditors] Failed to load tools:', error)
  } finally {
    loadingTools.value = false
  }
}

watch(
  () => props.toolPolicyMatrix,
  value => {
    if (!jsonErrors.toolPolicyMatrix) jsonDrafts.toolPolicyMatrix = JSON.stringify(value || {}, null, 2)
  },
  { immediate: true, deep: true },
)

watch(
  () => props.memoryPolicy,
  value => {
    if (!jsonErrors.memoryPolicy) jsonDrafts.memoryPolicy = JSON.stringify(value || {}, null, 2)
  },
  { immediate: true, deep: true },
)

watch(
  () => props.harnessPolicy,
  value => {
    if (!jsonErrors.harnessPolicy) jsonDrafts.harnessPolicy = JSON.stringify(value || {}, null, 2)
  },
  { immediate: true, deep: true },
)

watch(
  () => props.concurrencyPolicy,
  value => {
    if (!jsonErrors.concurrencyPolicy) jsonDrafts.concurrencyPolicy = JSON.stringify(value || {}, null, 2)
  },
  { immediate: true, deep: true },
)

watch(
  () => props.safetyPolicy,
  value => {
    if (!jsonErrors.safetyPolicy) jsonDrafts.safetyPolicy = JSON.stringify(value || {}, null, 2)
  },
  { immediate: true, deep: true },
)

const PolicyHeader = defineComponent({
  props: {
    mode: {
      type: String as PropType<PolicyMode>,
      required: true,
    },
    title: {
      type: String,
      required: true,
    },
  },
  emits: ['update:mode'],
  setup(policyProps, { emit: componentEmit }) {
    return () => h('div', { class: 'mb-3 flex flex-wrap items-center justify-between gap-2' }, [
      h('div', { class: 'text-sm font-semibold' }, policyProps.title),
      h('div', { class: 'join' }, [
        h('button', {
          class: [
            'btn btn-xs join-item',
            policyProps.mode === 'form' ? 'btn-primary' : 'btn-outline',
          ],
          type: 'button',
          onClick: () => componentEmit('update:mode', 'form'),
        }, '图形化'),
        h('button', {
          class: [
            'btn btn-xs join-item',
            policyProps.mode === 'json' ? 'btn-primary' : 'btn-outline',
          ],
          type: 'button',
          onClick: () => componentEmit('update:mode', 'json'),
        }, 'JSON'),
      ]),
    ])
  },
})

const JsonPolicyTextarea = defineComponent({
  props: {
    draft: {
      type: String,
      required: true,
    },
    error: {
      type: String,
      default: '',
    },
  },
  emits: ['input'],
  setup(jsonProps, { emit: componentEmit }) {
    return () => h('div', { class: 'space-y-2' }, [
      h('textarea', {
        class: [
          'textarea textarea-bordered min-h-[220px] w-full font-mono text-xs',
          jsonProps.error ? 'textarea-error' : '',
        ],
        value: jsonProps.draft,
        onInput: (event: Event) => componentEmit('input', (event.target as HTMLTextAreaElement).value),
      }),
      jsonProps.error
        ? h('div', { class: 'text-xs text-error' }, jsonProps.error)
        : h('div', { class: 'text-xs text-base-content/50' }, 'JSON 有效时会立即同步到当前 Team Profile。'),
    ])
  },
})

const NumberField = defineComponent({
  props: {
    label: {
      type: String,
      required: true,
    },
    value: {
      type: Number,
      required: true,
    },
  },
  emits: ['update:value'],
  setup(numberProps, { emit: componentEmit }) {
    return () => h('label', { class: 'form-control' }, [
      h('span', { class: 'label-text mb-1' }, numberProps.label),
      h('input', {
        class: 'input input-bordered input-sm',
        min: '1',
        type: 'number',
        value: numberProps.value,
        onInput: (event: Event) => {
          const value = Math.max(1, Math.floor(Number((event.target as HTMLInputElement).value) || 1))
          componentEmit('update:value', value)
        },
      }),
    ])
  },
})

const BooleanPolicySwitch = defineComponent({
  props: {
    checked: {
      type: Boolean,
      required: true,
    },
    label: {
      type: String,
      required: true,
    },
  },
  emits: ['update:checked'],
  setup(booleanProps, { emit: componentEmit }) {
    return () => h('label', { class: 'label cursor-pointer justify-start gap-3 rounded-lg border border-base-300 bg-base-200/30 px-3' }, [
      h('input', {
        checked: booleanProps.checked,
        class: 'toggle toggle-primary toggle-sm',
        type: 'checkbox',
        onChange: (event: Event) => componentEmit('update:checked', (event.target as HTMLInputElement).checked),
      }),
      h('span', { class: 'label-text' }, booleanProps.label),
    ])
  },
})

const categoryDisplayName = (category: string) => {
  const nameMap: Record<string, string> = {
    file_code: '文件与代码',
    terminal: '终端与运行环境',
    web_network: 'Web 与网络',
    security_recon: '安全侦察与扫描',
    vulnerability_research: '漏洞利用研究',
    collaboration: '任务与人机协作',
    agent_orchestration: 'Agent 编排',
    knowledge_extension: '知识与扩展',
    network: '网络',
    security: '安全',
    data: '数据',
    ai: 'AI',
    system: '系统',
    utility: '工具',
    mcp: 'MCP',
    plugin: '插件',
    workflow: '工作流',
    browser: '浏览器',
    recon: '侦察',
    scanning: '扫描',
    exploitation: '利用',
    monitoring: '监控',
    other: '其他',
  }
  return nameMap[category.toLowerCase()] || category
}

const categoryBadgeClass = (category: string) => {
  const map: Record<string, string> = {
    file_code: 'badge-info',
    terminal: 'badge-neutral',
    web_network: 'badge-primary',
    security_recon: 'badge-warning',
    vulnerability_research: 'badge-error',
    collaboration: 'badge-secondary',
    agent_orchestration: 'badge-accent',
    knowledge_extension: 'badge-success',
    network: 'badge-info',
    security: 'badge-error',
    data: 'badge-success',
    ai: 'badge-warning',
    system: 'badge-neutral',
    utility: 'badge-success',
    mcp: 'badge-primary',
    plugin: 'badge-secondary',
    workflow: 'badge-accent',
    browser: 'badge-primary',
    recon: 'badge-info',
    scanning: 'badge-accent',
    exploitation: 'badge-error',
    monitoring: 'badge-secondary',
    other: 'badge-ghost',
  }
  return map[category.toLowerCase()] || 'badge-ghost'
}

const categoryIcon = (category: string) => {
  const map: Record<string, string> = {
    file_code: 'fas fa-code',
    terminal: 'fas fa-terminal',
    web_network: 'fas fa-globe',
    security_recon: 'fas fa-binoculars',
    vulnerability_research: 'fas fa-bug',
    collaboration: 'fas fa-list-check',
    agent_orchestration: 'fas fa-code-branch',
    knowledge_extension: 'fas fa-lightbulb',
    network: 'fas fa-network-wired',
    security: 'fas fa-shield-alt',
    data: 'fas fa-database',
    ai: 'fas fa-brain',
    system: 'fas fa-cog',
    utility: 'fas fa-tools',
    mcp: 'fas fa-plug',
    plugin: 'fas fa-puzzle-piece',
    workflow: 'fas fa-project-diagram',
    browser: 'fas fa-window-maximize',
    recon: 'fas fa-binoculars',
    scanning: 'fas fa-radar',
    exploitation: 'fas fa-bug',
    monitoring: 'fas fa-satellite-dish',
    other: 'fas fa-tools',
  }
  return map[category.toLowerCase()] || 'fas fa-tools'
}

const ToolPolicyListPicker = defineComponent({
  props: {
    compact: {
      type: Boolean,
      default: false,
    },
    loading: {
      type: Boolean,
      default: false,
    },
    placeholder: {
      type: String,
      default: '',
    },
    title: {
      type: String,
      required: true,
    },
    tools: {
      type: Array as PropType<ToolMetadata[]>,
      required: true,
    },
    value: {
      type: Array as PropType<string[]>,
      required: true,
    },
  },
  emits: ['reload', 'update:value'],
  setup(pickerProps, { emit: componentEmit }) {
    const expanded = ref(false)
    const searchQuery = ref('')
    const selectedCategories = ref<string[]>([])
    const showSelectedOnly = ref(false)
    const customToolId = ref('')

    const selectedIds = computed(() => normalizeList(pickerProps.value))
    const selectedSet = computed(() => new Set(selectedIds.value))
    const toolById = computed(() => {
      const out = new Map<string, ToolMetadata>()
      pickerProps.tools.forEach(tool => out.set(tool.id, tool))
      return out
    })
    const allCategories = computed(() => {
      const out = new Set(pickerProps.tools.map(tool => tool.category || 'other'))
      out.add('plugin')
      return Array.from(out).sort()
    })
    const filteredTools = computed(() => {
      let tools = pickerProps.tools
      if (searchQuery.value.trim()) {
        const query = searchQuery.value.toLowerCase()
        tools = tools.filter(tool =>
          tool.id.toLowerCase().includes(query)
          || tool.name.toLowerCase().includes(query)
          || tool.description.toLowerCase().includes(query),
        )
      }
      if (showSelectedOnly.value) {
        tools = tools.filter(tool => selectedSet.value.has(tool.id))
      }
      if (selectedCategories.value.length > 0) {
        tools = tools.filter(tool => selectedCategories.value.includes(tool.category || 'other'))
      }
      return tools
    })

    const update = (ids: string[]) => {
      componentEmit('update:value', normalizeList(ids))
    }
    const toggleTool = (toolId: string) => {
      const current = new Set(selectedIds.value)
      if (current.has(toolId)) {
        current.delete(toolId)
      } else {
        current.add(toolId)
      }
      update(Array.from(current))
    }
    const removeTool = (toolId: string) => {
      update(selectedIds.value.filter(id => id !== toolId))
    }
    const addCustomTool = () => {
      const ids = parseListText(customToolId.value)
      if (ids.length === 0) return
      update([...selectedIds.value, ...ids])
      customToolId.value = ''
    }
    const toggleCategory = (category: string) => {
      selectedCategories.value = selectedCategories.value.includes(category)
        ? selectedCategories.value.filter(item => item !== category)
        : [...selectedCategories.value, category]
    }
    const clearFilters = () => {
      selectedCategories.value = []
      showSelectedOnly.value = false
    }
    const selectAllFiltered = () => {
      update([...selectedIds.value, ...filteredTools.value.map(tool => tool.id)])
    }
    const deselectAllFiltered = () => {
      const filteredIds = new Set(filteredTools.value.map(tool => tool.id))
      update(selectedIds.value.filter(id => !filteredIds.has(id)))
    }
    const selectedToolLabel = (toolId: string) => {
      const tool = toolById.value.get(toolId)
      return tool?.name || toolId
    }
    const selectedToolTitle = (toolId: string) => {
      const tool = toolById.value.get(toolId)
      return tool ? `${tool.id} · ${tool.description}` : toolId
    }

    return () => h('div', { class: 'form-control min-w-0' }, [
      h('div', { class: 'mb-1 flex items-center justify-between gap-2' }, [
        h('span', { class: 'label-text' }, pickerProps.title),
        h('div', { class: 'flex items-center gap-1' }, [
          h('span', { class: 'text-xs text-base-content/50' }, `${selectedIds.value.length} 项`),
          h('button', {
            class: 'btn btn-xs btn-ghost btn-circle',
            title: '刷新工具目录',
            type: 'button',
            onClick: () => componentEmit('reload'),
          }, [h('i', { class: ['fas fa-sync-alt text-xs', pickerProps.loading ? 'animate-spin' : ''] })]),
          h('button', {
            class: ['btn btn-xs', expanded.value ? 'btn-primary' : 'btn-outline'],
            type: 'button',
            onClick: () => { expanded.value = !expanded.value },
          }, expanded.value ? '收起' : '选择工具'),
        ]),
      ]),
      h('div', { class: 'min-h-[2.5rem] rounded-lg border border-base-300 bg-base-100 px-2 py-2' }, [
        selectedIds.value.length > 0
          ? h('div', { class: 'flex flex-wrap gap-2' }, selectedIds.value.map(toolId => h('span', {
              key: toolId,
              class: 'badge badge-primary gap-1 max-w-full',
              title: selectedToolTitle(toolId),
            }, [
              h('span', { class: 'max-w-[14rem] truncate' }, selectedToolLabel(toolId)),
              h('button', {
                class: 'btn btn-ghost btn-circle btn-xs h-4 min-h-0 w-4',
                type: 'button',
                onClick: () => removeTool(toolId),
              }, [h('i', { class: 'fas fa-times text-[10px]' })]),
            ])))
          : h('span', { class: 'text-sm text-base-content/40' }, pickerProps.placeholder || '未选择工具'),
      ]),
      expanded.value
        ? h('div', { class: 'mt-2 rounded-lg border border-base-300 bg-base-100 p-3' }, [
            h('div', { class: 'relative mb-2' }, [
              h('input', {
                class: 'input input-bordered input-sm w-full pr-8',
                placeholder: '搜索工具名称、ID 或描述...',
                type: 'text',
                value: searchQuery.value,
                onInput: (event: Event) => { searchQuery.value = (event.target as HTMLInputElement).value },
              }),
              h('i', { class: 'fas fa-search absolute right-3 top-1/2 -translate-y-1/2 text-xs text-base-content/50' }),
            ]),
            h('div', { class: 'mb-2 flex flex-wrap items-center gap-2' }, [
              h('button', {
                class: ['btn btn-xs', selectedCategories.value.length === 0 && !showSelectedOnly.value ? 'btn-primary' : 'btn-ghost'],
                type: 'button',
                onClick: clearFilters,
              }, '全部'),
              h('button', {
                class: ['btn btn-xs', showSelectedOnly.value ? 'btn-primary' : 'btn-ghost'],
                type: 'button',
                onClick: () => {
                  showSelectedOnly.value = !showSelectedOnly.value
                  if (showSelectedOnly.value) selectedCategories.value = []
                },
              }, `已选 (${selectedIds.value.length})`),
              ...allCategories.value.map(category => h('button', {
                key: category,
                class: ['btn btn-xs', selectedCategories.value.includes(category) ? 'btn-primary' : 'btn-ghost'],
                type: 'button',
                onClick: () => toggleCategory(category),
              }, [
                h('i', { class: `${categoryIcon(category)} mr-1` }),
                categoryDisplayName(category),
              ])),
            ]),
            h('div', { class: 'mb-2 flex flex-wrap justify-end gap-2' }, [
              h('button', {
                class: 'btn btn-xs btn-outline btn-success',
                type: 'button',
                onClick: selectAllFiltered,
              }, [h('i', { class: 'fas fa-check-double mr-1' }), '全选']),
              h('button', {
                class: 'btn btn-xs btn-outline btn-error',
                type: 'button',
                onClick: deselectAllFiltered,
              }, [h('i', { class: 'fas fa-times mr-1' }), '取消全选']),
            ]),
            h('div', { class: ['space-y-1 overflow-y-auto rounded border border-base-300 p-2', pickerProps.compact ? 'max-h-48' : 'max-h-64'] }, [
              pickerProps.loading
                ? h('div', { class: 'flex justify-center py-6' }, [h('span', { class: 'loading loading-spinner loading-md' })])
                : filteredTools.value.length > 0
                  ? filteredTools.value.map(tool => h('label', {
                      key: tool.id,
                      class: 'flex cursor-pointer items-start gap-3 rounded px-2 py-2 hover:bg-base-200',
                    }, [
                      h('input', {
                        checked: selectedSet.value.has(tool.id),
                        class: 'checkbox checkbox-primary checkbox-sm mt-1',
                        type: 'checkbox',
                        onChange: () => toggleTool(tool.id),
                      }),
                      h('div', { class: 'min-w-0 flex-1' }, [
                        h('div', { class: 'flex min-w-0 flex-wrap items-center gap-2' }, [
                          h('span', { class: 'truncate text-sm font-medium' }, tool.name || tool.id),
                          h('span', { class: ['badge badge-xs', categoryBadgeClass(tool.category || 'other')] }, categoryDisplayName(tool.category || 'other')),
                        ]),
                        h('div', { class: 'mt-1 break-words text-xs leading-5 text-base-content/60' }, tool.description || tool.id),
                      ]),
                    ]))
                  : h('div', { class: 'py-6 text-center text-sm text-base-content/50' }, '没有匹配的工具'),
            ]),
            h('div', { class: 'mt-2 flex flex-col gap-2 sm:flex-row' }, [
              h('input', {
                class: 'input input-bordered input-sm flex-1',
                placeholder: '添加自定义/内部工具 ID，支持逗号分隔',
                type: 'text',
                value: customToolId.value,
                onInput: (event: Event) => { customToolId.value = (event.target as HTMLInputElement).value },
                onKeydown: (event: KeyboardEvent) => {
                  if (event.key !== 'Enter') return
                  event.preventDefault()
                  addCustomTool()
                },
              }),
              h('button', {
                class: 'btn btn-sm btn-outline',
                type: 'button',
                onClick: addCustomTool,
              }, '添加'),
            ]),
          ])
        : null,
    ])
  },
})

onMounted(() => {
  void loadTools()
})
</script>

<style scoped>
.policy-card {
  @apply rounded-xl border border-base-300 bg-base-100 p-4;
}
</style>
