<template>
  <div class="parallel-result rounded-lg border border-base-300 bg-base-100 overflow-hidden">
    <div class="flex flex-wrap items-center gap-2 border-b border-base-300 bg-base-200/50 px-4 py-3">
      <i class="fas fa-layer-group text-primary"></i>
      <div class="min-w-0 flex-1">
        <div class="text-sm font-semibold">多模型并行执行</div>
        <div class="text-xs text-base-content/60">{{ progressText }}</div>
      </div>
      <div class="join">
        <button class="btn btn-xs join-item" :class="viewMode === 'grid' ? 'btn-primary' : 'btn-ghost'" @click="viewMode = 'grid'">
          分栏
        </button>
        <button class="btn btn-xs join-item" :class="viewMode === 'focus' ? 'btn-primary' : 'btn-ghost'" @click="viewMode = 'focus'">
          聚焦
        </button>
        <button class="btn btn-xs join-item" :class="viewMode === 'table' ? 'btn-primary' : 'btn-ghost'" @click="viewMode = 'table'">
          表格
        </button>
      </div>
      <span class="badge badge-sm" :class="runBadgeClass">{{ run.status }}</span>
    </div>

    <div class="grid grid-cols-2 gap-2 border-b border-base-300 px-4 py-3 text-xs md:grid-cols-6">
      <div v-for="item in runOverviewItems" :key="item.label" class="rounded border border-base-300 bg-base-200/40 px-2 py-1">
        <div class="text-base-content/50">{{ item.label }}</div>
        <div class="font-semibold text-base-content/80">{{ item.value }}</div>
      </div>
    </div>

    <div v-if="run.aggregationMode === 'judge'" class="border-b border-base-300 px-4 py-3">
      <div class="mb-2 flex items-center gap-2 text-sm font-medium">
        <i class="fas fa-scale-balanced text-secondary"></i>
        <span>Judge 汇总</span>
        <span class="badge badge-xs" :class="judgeBadgeClass">{{ judgeLabel }}</span>
      </div>
      <div v-if="judgeStructured" class="grid gap-3 md:grid-cols-[1fr_2fr]">
        <div class="rounded border border-base-300 bg-base-200/40 p-3 text-sm">
          <div class="text-xs text-base-content/50">最佳模型</div>
          <div class="font-semibold">{{ judgeStructured.best_model || 'N/A' }}</div>
          <div class="mt-2 text-xs text-base-content/50">置信度</div>
          <div class="font-semibold">{{ formatConfidence(judgeStructured.confidence) }}</div>
        </div>
        <div class="rounded border border-base-300 bg-base-100 p-3">
          <div class="mb-1 text-xs font-medium text-base-content/60">最终建议</div>
          <MarkdownRenderer :content="judgeStructured.final_answer || judgeStructured.recommendation || ''" />
        </div>
        <div v-if="judgeStructured.disagreements?.length" class="rounded border border-base-300 bg-base-100 p-3 md:col-span-2">
          <div class="mb-2 text-xs font-medium text-base-content/60">冲突点</div>
          <ul class="list-disc space-y-1 pl-4 text-sm">
            <li v-for="(item, index) in judgeStructured.disagreements" :key="index">{{ item }}</li>
          </ul>
        </div>
      </div>
      <div v-else-if="run.judgeContent" class="prose prose-sm max-w-none">
        <MarkdownRenderer :content="run.judgeContent" />
      </div>
      <div v-else-if="run.judgeError" class="text-sm text-error">{{ run.judgeError }}</div>
      <div v-else class="text-sm text-base-content/60">等待所有模型完成后自动汇总。</div>
    </div>

    <div v-if="viewMode === 'table'" class="overflow-x-auto p-3">
      <table class="table table-sm">
        <thead>
          <tr>
            <th>模型</th>
            <th>状态</th>
            <th>首次响应</th>
            <th>Token</th>
            <th>速度</th>
            <th>成本</th>
            <th>耗时</th>
            <th>工具</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in run.items" :key="item.modelRunId">
            <td class="font-medium">{{ item.provider }}/{{ item.model }}</td>
            <td><span class="badge badge-xs" :class="itemBadgeClass(item.status)">{{ item.status }}</span></td>
            <td>{{ metricMap(item).first }}</td>
            <td>{{ metricMap(item).tokens }}</td>
            <td>{{ metricMap(item).rate }}</td>
            <td>{{ metricMap(item).cost }}</td>
            <td>{{ metricMap(item).duration }}</td>
            <td>{{ item.toolCount }}</td>
            <td class="text-right">
              <button class="btn btn-ghost btn-xs" @click="openModelDialog(item)">查看</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-else-if="viewMode === 'focus'" class="p-3">
      <div class="mb-3 flex flex-wrap gap-2">
        <button
          v-for="item in run.items"
          :key="item.modelRunId"
          class="btn btn-xs"
          :class="focusedModelRunId === item.modelRunId ? 'btn-primary' : 'btn-outline'"
          @click="focusedModelRunId = item.modelRunId"
        >
          {{ item.model }}
        </button>
      </div>
      <ParallelModelCard
        v-if="focusedItem"
        :item="focusedItem"
        :metrics="modelMetricItems(focusedItem)"
        :status-class="itemBadgeClass(focusedItem.status)"
        full-height
        @open="openModelDialog(focusedItem)"
        @cancel="cancelModelRun(focusedItem)"
        @retry="retryModelRun(focusedItem)"
      />
    </div>

    <div v-else class="grid gap-3 p-3 lg:grid-cols-2">
      <ParallelModelCard
        v-for="item in run.items"
        :key="item.modelRunId"
        :item="item"
        :metrics="modelMetricItems(item)"
        :status-class="itemBadgeClass(item.status)"
        @open="openModelDialog(item)"
        @cancel="cancelModelRun(item)"
        @retry="retryModelRun(item)"
      />
    </div>

    <dialog class="modal" :open="!!selectedItem">
      <div class="modal-box h-[85vh] max-w-5xl overflow-hidden p-0">
        <div class="flex items-center gap-3 border-b border-base-300 px-4 py-3">
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-semibold">
              {{ selectedItem?.provider }}/{{ selectedItem?.model }}
            </div>
            <div class="text-xs text-base-content/60">{{ selectedItem?.modelRunId }}</div>
          </div>
          <span v-if="selectedItem" class="badge badge-sm" :class="itemBadgeClass(selectedItem.status)">
            {{ selectedItem.status }}
          </span>
          <button class="btn btn-ghost btn-sm btn-square" @click="closeModelDialog">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div v-if="selectedItem" class="grid h-[calc(85vh-57px)] grid-rows-[auto_auto_1fr] overflow-hidden">
          <div class="grid grid-cols-2 gap-2 border-b border-base-300 bg-base-200/30 p-3 md:grid-cols-5">
            <div
              v-for="metric in modelMetricItems(selectedItem)"
              :key="metric.label"
              class="rounded border border-base-300 bg-base-100 px-3 py-2 text-xs"
            >
              <div class="text-base-content/50">{{ metric.label }}</div>
              <div class="font-semibold text-base-content">{{ metric.value }}</div>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2 border-b border-base-300 p-3">
            <input
              v-model="dialogSearch"
              class="input input-sm input-bordered min-w-48 flex-1"
              placeholder="搜索当前模型输出"
            />
            <select v-model="dialogFilter" class="select select-sm select-bordered">
              <option value="all">全部</option>
              <option value="tools">只看工具</option>
              <option value="errors">只看错误</option>
            </select>
            <button class="btn btn-sm btn-outline" @click="exportSelectedModel">
              导出
            </button>
          </div>

          <div class="overflow-hidden p-4">
            <ParallelModelTimeline
              :item="selectedItem"
              :filter="dialogFilter"
              :search="dialogSearch"
            />
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop" @submit.prevent="closeModelDialog">
        <button @click="closeModelDialog">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, ref, type PropType } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import MarkdownRenderer from './MarkdownRenderer.vue'
import ParallelModelTimeline from './ParallelModelTimeline.vue'
import {
  buildAgentSessionStats,
  formatSessionDuration,
  formatTokenRate,
} from './agentSessionStatsSupport'
import type {
  ParallelModelState,
  ParallelModelStatus,
  ParallelRunState,
} from '@/composables/agentParallelEventSupport'

type ViewMode = 'grid' | 'focus' | 'table'

interface MetricItem {
  label: string
  value: string
}

const props = defineProps<{
  run: ParallelRunState
}>()

const viewMode = ref<ViewMode>('grid')
const selectedItem = ref<ParallelModelState | null>(null)
const focusedModelRunId = ref('')
const dialogSearch = ref('')
const dialogFilter = ref<'all' | 'tools' | 'errors'>('all')

const terminalStatuses = ['succeeded', 'failed', 'cancelled']

const progressText = computed(() => {
  const done = props.run.items.filter((item) => terminalStatuses.includes(item.status)).length
  return `${done}/${props.run.items.length} 个模型完成`
})

const focusedItem = computed(() => {
  const explicit = props.run.items.find((item) => item.modelRunId === focusedModelRunId.value)
  return explicit || props.run.items[0] || null
})

const runOverviewItems = computed(() => {
  const succeeded = props.run.items.filter((item) => item.status === 'succeeded').length
  const failed = props.run.items.filter((item) => item.status === 'failed').length
  const running = props.run.items.filter((item) => item.status === 'running' || item.status === 'pending').length
  const totalTokens = props.run.items.reduce((sum, item) => sum + (item.inputTokens || 0) + (item.outputTokens || 0), 0)
  const totalCost = props.run.items.reduce((sum, item) => sum + (item.costUsd || 0), 0)
  const totalTools = props.run.items.reduce((sum, item) => sum + item.toolCount, 0)
  const fastest = props.run.items
    .map((item) => ({ item, duration: modelStats(item)?.duration_ms || 0 }))
    .filter((entry) => entry.duration > 0)
    .sort((a, b) => a.duration - b.duration)[0]
  return [
    { label: '成功', value: String(succeeded) },
    { label: '失败', value: String(failed) },
    { label: '运行中', value: String(running) },
    { label: '总 Token', value: formatNumber(totalTokens) },
    { label: '总成本', value: totalCost > 0 ? `$${totalCost.toFixed(6)}` : 'N/A' },
    { label: '工具调用', value: String(totalTools) },
    { label: '最快模型', value: fastest ? fastest.item.model : 'N/A' },
  ]
})

const runBadgeClass = computed(() => {
  switch (props.run.status) {
    case 'succeeded':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    case 'cancelled':
      return 'badge-warning'
    default:
      return 'badge-info'
  }
})

const judgeLabel = computed(() => {
  switch (props.run.judgeStatus) {
    case 'succeeded':
      return '完成'
    case 'failed':
      return '失败'
    case 'running':
      return '汇总中'
    case 'pending':
      return '等待'
    default:
      return '未启用'
  }
})

const judgeBadgeClass = computed(() => {
  switch (props.run.judgeStatus) {
    case 'succeeded':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    case 'running':
      return 'badge-info'
    default:
      return 'badge-ghost'
  }
})

const judgeStructured = computed(() => {
  const raw = props.run.judgeContent?.trim()
  if (!raw) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
})

const formatConfidence = (value: unknown) => {
  const numeric = Number(value)
  if (!Number.isFinite(numeric)) return 'N/A'
  return `${Math.round(Math.max(0, Math.min(1, numeric)) * 100)}%`
}

const itemBadgeClass = (status: ParallelModelStatus) => {
  switch (status) {
    case 'succeeded':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    case 'cancelled':
      return 'badge-warning'
    case 'running':
      return 'badge-info'
    default:
      return 'badge-ghost'
  }
}

const openModelDialog = (item: ParallelModelState) => {
  selectedItem.value = item
}

const closeModelDialog = () => {
  selectedItem.value = null
  dialogSearch.value = ''
  dialogFilter.value = 'all'
}

const exportSelectedModel = () => {
  if (!selectedItem.value) return
  const item = selectedItem.value
  const body = [
    `# ${item.provider}/${item.model}`,
    '',
    `status: ${item.status}`,
    `tokens: ${(item.inputTokens || 0) + (item.outputTokens || 0)}`,
    `cost: ${item.costUsd ? `$${item.costUsd.toFixed(6)}` : 'N/A'}`,
    '',
    item.content || '',
  ].join('\n')
  void navigator.clipboard.writeText(body)
}

const modelStats = (item: ParallelModelState) => {
  const endedAt = item.completedAtMs || (item.status === 'running' ? Date.now() : null)
  return buildAgentSessionStats({
    startedAt: item.startedAtMs,
    endedAt,
    firstResponseAt: item.firstResponseMs && item.startedAtMs
      ? Number(item.startedAtMs) + item.firstResponseMs
      : null,
    inputTokens: item.inputTokens || 0,
    outputTokens: item.outputTokens || 0,
  })
}

const formatNumber = (value: unknown) => {
  const numberValue = Number(value)
  if (!Number.isFinite(numberValue)) return '0'
  return Math.floor(numberValue).toLocaleString()
}

const metricMap = (item: ParallelModelState) => {
  const stats = modelStats(item)
  const first = formatSessionDuration(item.firstResponseMs || stats?.first_response_ms) || 'N/A'
  const duration = formatSessionDuration(stats?.duration_ms) || 'N/A'
  const rateRaw = formatTokenRate(stats?.tokens_per_second)
  const inputTokens = item.inputTokens || stats?.input_tokens || 0
  const outputTokens = item.outputTokens || stats?.output_tokens || 0
  return {
    first,
    tokens: `${formatNumber(inputTokens + outputTokens)} total`,
    io: `${formatNumber(inputTokens)} / ${formatNumber(outputTokens)}`,
    rate: rateRaw ? `${rateRaw} tok/s` : 'N/A',
    duration,
    cost: item.costUsd && item.costUsd > 0 ? `$${item.costUsd.toFixed(6)}` : 'N/A',
  }
}

const modelMetricItems = (item: ParallelModelState): MetricItem[] => {
  const metrics = metricMap(item)
  const items = [
    { label: '首次响应', value: metrics.first },
    { label: 'Token 用量', value: metrics.tokens },
    { label: '输入 / 输出', value: metrics.io },
    { label: 'Token 速度', value: metrics.rate },
    { label: '成本', value: metrics.cost },
    { label: '总耗时', value: metrics.duration },
  ]
  const anomaly = anomalyLabel(item)
  return anomaly ? [{ label: '提示', value: anomaly }, ...items] : items
}

const anomalyLabel = (item: ParallelModelState) => {
  const stats = modelStats(item)
  const durations = props.run.items
    .map((candidate) => modelStats(candidate)?.duration_ms || 0)
    .filter((value) => value > 0)
  const averageDuration = durations.length
    ? durations.reduce((sum, value) => sum + value, 0) / durations.length
    : 0
  const totalTokens = (item.inputTokens || 0) + (item.outputTokens || 0)
  const tokenCounts = props.run.items.map((candidate) => (candidate.inputTokens || 0) + (candidate.outputTokens || 0))
  const averageTokens = tokenCounts.length
    ? tokenCounts.reduce((sum, value) => sum + value, 0) / tokenCounts.length
    : 0
  if (stats?.duration_ms && averageDuration > 0 && stats.duration_ms >= averageDuration * 2) return '耗时偏高'
  if (averageTokens > 0 && totalTokens >= averageTokens * 2) return 'Token 偏高'
  if (item.status === 'failed') return '执行失败'
  return ''
}

const cancelModelRun = (item: ParallelModelState) => {
  void invoke('cancel_ai_parallel_model_run', {
    request: {
      parallel_run_id: props.run.id,
      model_run_id: item.modelRunId,
    },
  })
}

const retryModelRun = (item: ParallelModelState) => {
  void invoke('retry_ai_parallel_model_run', {
    request: {
      parallel_run_id: props.run.id,
      model_run_id: item.modelRunId,
      provider: item.provider,
      model: item.model,
    },
  })
}

const ParallelModelCard = defineComponent({
  name: 'ParallelModelCard',
  props: {
    item: { type: Object as PropType<ParallelModelState>, required: true },
    metrics: { type: Array as PropType<MetricItem[]>, required: true },
    statusClass: { type: String, required: true },
    fullHeight: { type: Boolean, default: false },
  },
  emits: ['open', 'cancel', 'retry'],
  setup(cardProps, { emit }) {
    return () => h('article', {
      class: [
        'min-w-0 overflow-hidden rounded-md border border-base-300 bg-base-100',
        cardProps.fullHeight ? 'h-[620px]' : '',
      ],
    }, [
      h('div', { class: 'sticky top-0 z-10 border-b border-base-300 bg-base-100/95 px-3 py-2 backdrop-blur' }, [
        h('div', { class: 'flex items-center gap-2' }, [
          h('span', { class: 'truncate text-sm font-semibold' }, `${cardProps.item.provider}/${cardProps.item.model}`),
          h('button', {
            class: 'btn btn-ghost btn-xs btn-square',
            title: '单独查看',
            onClick: () => emit('open'),
          }, [h('i', { class: 'fas fa-up-right-from-square' })]),
          h('button', {
            class: 'btn btn-ghost btn-xs btn-square',
            title: '查看该模型任务',
            onClick: () => window.dispatchEvent(new CustomEvent('agent:parallel-task-source-focus', {
              detail: { sourceKey: `parallel:${cardProps.item.modelRunId}` },
            })),
          }, [h('i', { class: 'fas fa-list-check' })]),
          h('button', {
            class: 'btn btn-ghost btn-xs btn-square',
            title: '重跑该模型',
            onClick: () => emit('retry'),
          }, [h('i', { class: 'fas fa-rotate-right' })]),
          h('button', {
            class: 'btn btn-ghost btn-xs btn-square',
            title: '停止该模型',
            disabled: !['pending', 'running'].includes(cardProps.item.status),
            onClick: () => emit('cancel'),
          }, [h('i', { class: 'fas fa-stop' })]),
          h('span', { class: ['badge badge-xs ml-auto', cardProps.statusClass] }, cardProps.item.status),
        ]),
        h('div', { class: 'mt-2 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-base-content/60' },
          cardProps.metrics.map((metric) => h('span', { class: 'inline-flex gap-1' }, [
            h('span', { class: 'text-base-content/40' }, `${metric.label}:`),
            h('span', { class: 'font-medium text-base-content/80' }, metric.value),
          ])),
        ),
      ]),
      h('div', {
        class: ['overflow-hidden px-3 py-3', cardProps.fullHeight ? 'h-[536px]' : 'h-96'],
      }, [h(ParallelModelTimeline, { item: cardProps.item })]),
    ])
  },
})
</script>
