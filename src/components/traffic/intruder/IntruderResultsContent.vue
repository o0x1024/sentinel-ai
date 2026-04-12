<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div class="flex items-center gap-3 border-b border-base-300 px-6 py-4">
      <div class="min-w-0 flex-1">
        <h3 class="truncate text-lg font-semibold">
          {{ workspaceName }}
        </h3>
        <p class="text-sm text-base-content/70">
          {{ progress.completed }}/{{ progress.total }} {{ $t('trafficAnalysis.intruder.labels.completed') }}
          <span v-if="progress.failed > 0">· {{ progress.failed }} {{ $t('trafficAnalysis.intruder.labels.failed') }}</span>
          <span v-if="isRunning">· {{ $t('trafficAnalysis.intruder.labels.running') }}</span>
        </p>
      </div>

      <div class="tabs tabs-boxed bg-base-200">
        <button class="tab" :class="{ 'tab-active': activeView === 'results' }" type="button" @click="activeView = 'results'">
          {{ $t('trafficAnalysis.intruder.sections.results') }}
        </button>
        <button class="tab" :class="{ 'tab-active': activeView === 'positions' }" type="button" @click="activeView = 'positions'">
          {{ $t('trafficAnalysis.intruder.sections.positions') }}
        </button>
      </div>

      <slot name="actions" />
    </div>

    <div v-if="activeView === 'results'" class="flex min-h-0 flex-1 flex-col">
      <div class="grid gap-2 border-b border-base-300 bg-base-100 px-6 py-3 xl:grid-cols-2">
        <IntruderResultFilterCard
          kind="capture"
          :label="$t('trafficAnalysis.intruder.labels.captureFilterTitle')"
          :filter="captureFilter"
          :grep-match-rules="grepMatchRules"
          :columns="allColumns"
          @update:filter="$emit('update:captureFilter', $event)"
        />

        <IntruderResultFilterCard
          kind="view"
          :label="$t('trafficAnalysis.intruder.labels.viewFilterTitle')"
          :filter="viewFilter"
          :grep-match-rules="grepMatchRules"
          :columns="allColumns"
          @update:filter="$emit('update:viewFilter', $event)"
        />
      </div>

      <div class="flex items-center justify-end border-b border-base-300 px-6 py-2">
        <details class="dropdown dropdown-end">
          <summary class="btn btn-sm btn-ghost">
            <i class="fas fa-columns"></i>
            {{ $t('trafficAnalysis.intruder.labels.columns') }}
          </summary>
          <div class="dropdown-content z-10 mt-2 w-72 rounded-lg border border-base-300 bg-base-100 p-3 shadow-xl">
            <div class="mb-3 flex gap-2">
              <button class="btn btn-xs btn-ghost" type="button" @click="selectAllColumns">
                {{ $t('trafficAnalysis.intruder.actions.selectAll') }}
              </button>
              <button class="btn btn-xs btn-ghost" type="button" @click="showBaseColumnsOnly">
                {{ $t('trafficAnalysis.intruder.actions.baseOnly') }}
              </button>
              <button class="btn btn-xs btn-ghost" type="button" @click="resetVisibleColumns">
                {{ $t('trafficAnalysis.intruder.actions.reset') }}
              </button>
            </div>
            <div class="space-y-2">
              <div v-for="column in chooserColumns" :key="`column-toggle-${column.key}`" class="flex items-center gap-2 text-sm">
                <input
                  :checked="visibleColumnKeys.includes(column.key)"
                  type="checkbox"
                  class="checkbox checkbox-xs"
                  @change="toggleVisibleColumn(column.key, ($event.target as HTMLInputElement).checked)"
                />
                <span class="min-w-0 flex-1 truncate">{{ column.label }}</span>
                <button
                  class="btn btn-ghost btn-xs"
                  type="button"
                  :disabled="getVisibleColumnIndex(column.key) <= 0"
                  @click="moveColumn(column.key, 'up')"
                >
                  <i class="fas fa-arrow-up"></i>
                </button>
                <button
                  class="btn btn-ghost btn-xs"
                  type="button"
                  :disabled="getVisibleColumnIndex(column.key) === -1 || getVisibleColumnIndex(column.key) >= visibleColumnKeys.length - 1"
                  @click="moveColumn(column.key, 'down')"
                >
                  <i class="fas fa-arrow-down"></i>
                </button>
              </div>
            </div>
          </div>
        </details>
      </div>

      <div class="min-h-0 flex-1 overflow-hidden">
        <div class="h-72 overflow-auto border-b border-base-300">
          <table class="table table-pin-rows table-sm">
            <thead>
              <tr>
                <th v-for="column in displayedColumns" :key="column.key">
                  <button class="font-semibold" type="button" @click="toggleSort(column.key)">
                    {{ column.label }} {{ renderSortMarker(column.key) }}
                  </button>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="result in visibleResults"
                :key="result.id"
                class="cursor-pointer"
                :class="{ 'bg-primary/10': result.id === selectedResultId }"
                @click="$emit('selectResult', result.id)"
                @contextmenu.prevent="showResultContextMenu($event, result.id)"
              >
                <td v-for="column in displayedColumns" :key="`${result.id}-${column.key}`">
                  <template v-if="column.key === 'index'">
                    <span class="font-mono">{{ result.index - 1 }}</span>
                    <span v-if="result.isBaseline" class="ml-2 badge badge-outline badge-xs">{{ $t('trafficAnalysis.intruder.labels.baseline') }}</span>
                  </template>
                  <span
                    v-else-if="column.kind === 'match'"
                    class="badge badge-xs"
                    :class="getColumnBadgeClass(result, column.key)"
                  >
                    {{ getColumnDisplayValue(result, column.key) }}
                  </span>
                  <span
                    v-else-if="column.key === 'payloadSummary'"
                    class="block max-w-64 truncate font-mono text-[11px]"
                    :title="String(getColumnDisplayValue(result, column.key))"
                  >
                    {{ getColumnDisplayValue(result, column.key) }}
                  </span>
                  <span
                    v-else-if="column.key === 'error' || column.kind === 'extract'"
                    class="block max-w-56 truncate font-mono text-[11px]"
                    :title="String(getColumnDisplayValue(result, column.key))"
                  >
                    {{ getColumnDisplayValue(result, column.key) }}
                  </span>
                  <span v-else class="font-mono text-[11px]">
                    {{ getColumnDisplayValue(result, column.key) }}
                  </span>
                </td>
              </tr>
              <tr v-if="!visibleResults.length">
                <td :colspan="displayedColumns.length" class="py-10 text-center text-sm text-base-content/60">
                  {{ $t('trafficAnalysis.intruder.empty.noResults') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div
          v-if="resultContextMenu.visible"
          class="fixed z-50 min-w-48 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
          :style="{ left: `${resultContextMenu.x}px`, top: `${resultContextMenu.y}px` }"
          @click.stop
        >
          <TrafficContextMenuSections
            :sections="resultContextMenuSections"
            label-prefix="trafficAnalysis.intruder.contextMenu"
          />
        </div>

        <div class="grid h-[calc(100%-18rem)] min-h-0 grid-cols-2">
          <div
            class="min-h-0 border-r border-base-300"
            @contextmenu.capture.prevent="showSelectedResultContextMenu($event)"
          >
            <div class="border-b border-base-300 bg-base-200 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.request') }}
            </div>
            <div class="h-[calc(100%-2.5rem)]">
              <HttpMessageSurface
                :model-value="selectedResult?.rawRequest || ''"
                custom-context-menu
                message-type="request"
                readonly
                show-search-bar
                display-mode="raw"
                :state-key="selectedResult ? `intruder:request:${selectedResult.id}` : ''"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                @contextmenu.capture.prevent="showSelectedResultContextMenu($event)"
              />
            </div>
          </div>

          <div class="min-h-0" @contextmenu.capture.prevent="showSelectedResultContextMenu($event)">
            <div class="border-b border-base-300 bg-base-200 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              <div class="flex items-center justify-between gap-2">
                <span>{{ $t('trafficAnalysis.intruder.labels.response') }}</span>
                <div v-if="diffSummary" class="flex flex-wrap gap-2 text-[11px] normal-case tracking-normal">
                  <span class="badge badge-outline" :class="deltaClass(diffSummary.statusDelta)">{{ $t('trafficAnalysis.intruder.labels.status') }} {{ formatDelta(diffSummary.statusDelta) }}</span>
                  <span class="badge badge-outline" :class="deltaClass(diffSummary.lengthDelta)">{{ $t('trafficAnalysis.intruder.labels.length') }} {{ formatDelta(diffSummary.lengthDelta) }}</span>
                  <span class="badge badge-outline" :class="deltaClass(diffSummary.timeDelta)">{{ $t('trafficAnalysis.intruder.labels.time') }} {{ formatDelta(diffSummary.timeDelta) }}</span>
                  <span class="badge badge-outline">{{ $t('trafficAnalysis.intruder.labels.changedLines') }} {{ diffSummary.changedLines }}</span>
                </div>
                <div
                  v-if="selectedResult?.redirectCount"
                  class="flex flex-wrap gap-2 text-[11px] normal-case tracking-normal"
                >
                  <span class="badge badge-outline">
                    {{ $t('trafficAnalysis.intruder.labels.redirects') }} {{ selectedResult.redirectCount }}
                  </span>
                  <span
                    class="badge badge-outline max-w-72 truncate"
                    :title="selectedResult.finalUrl"
                  >
                    {{ selectedResult.finalUrl }}
                  </span>
                </div>
              </div>
            </div>
            <div class="h-[calc(100%-2.5rem)]">
              <HttpMessageSurface
                :model-value="selectedResult?.rawResponse || selectedResult?.error || ''"
                custom-context-menu
                message-type="response"
                readonly
                show-search-bar
                display-mode="raw"
                :state-key="selectedResult ? `intruder:response:${selectedResult.id}` : ''"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                @contextmenu.capture.prevent="showSelectedResultContextMenu($event)"
              />
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="flex min-h-0 flex-1 flex-col">
      <div class="border-b border-base-300 bg-base-200 px-6 py-3 text-sm text-base-content/70">
        {{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.detectedPositions') }}
      </div>
      <div class="grid min-h-0 flex-1 grid-cols-[20rem_1fr]">
        <div class="overflow-auto border-r border-base-300 p-4">
          <div class="space-y-2">
            <div
              v-for="position in positions"
              :key="position.index"
              class="rounded-lg border border-base-300 px-3 py-2"
            >
              <div class="text-xs text-base-content/60">#{{ position.index + 1 }}</div>
              <div class="truncate font-mono text-sm">{{ position.preview || '-' }}</div>
            </div>
          </div>
        </div>
        <div class="min-h-0">
          <HttpMessageSurface
            :model-value="requestText"
            message-type="request"
            readonly
            show-search-bar
            marker-mode="intruder"
            display-mode="raw"
            :state-key="`intruder:positions:${workspaceName}`"
            :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
            :search-next-title="$t('trafficAnalysis.messageSearch.next')"
            :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
            :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
            :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
            :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
            :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
            :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import { dialog } from '@/composables/useDialog'
import TrafficContextMenuSections from '@/components/traffic/TrafficContextMenuSections.vue'
import { buildTrafficRequestActionMenuItems } from '@/components/traffic/trafficRequestActionMenuSupport'
import { buildTrafficRequestContextMenuSections } from '@/components/traffic/trafficRequestContextMenuSupport'
import { useTrafficSendTargets } from '@/components/traffic/trafficSendTargets'
import { buildTrafficRequestSendMenuItems } from '@/components/traffic/trafficSendMenuSupport'
import {
  buildIntruderResultCurlCommand,
  resolveIntruderResultRequestUrl,
} from '@/components/traffic/trafficIntruderResultRequestSupport'
import IntruderResultFilterCard from './IntruderResultFilterCard.vue'
import { INTRUDER_BASE_RESULT_COLUMNS, getIntruderResultColumnValue, getIntruderResultColumns } from './analysis'
import {
  buildIntruderDiffSummary,
  matchesIntruderResultFilter,
  sortIntruderResults,
  summarizeIntruderResultFilter,
} from './results'
import type {
  IntruderAttackProgress,
  IntruderAttackResult,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPosition,
  IntruderResultFilter,
  IntruderResultSort,
} from './types'

const props = defineProps<{
  workspaceName: string
  requestText: string
  positions: IntruderPosition[]
  results: IntruderAttackResult[]
  selectedResultId: string | null
  selectedResult: IntruderAttackResult | null
  progress: IntruderAttackProgress
  isRunning: boolean
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  grepPayloadSettings: IntruderGrepPayloadSettings
  visibleColumns: string[]
}>()

const emit = defineEmits<{
  (e: 'selectResult', id: string): void
  (e: 'update:captureFilter', value: IntruderResultFilter): void
  (e: 'update:viewFilter', value: IntruderResultFilter): void
  (e: 'update:sort', value: IntruderResultSort): void
  (e: 'update:visibleColumns', value: string[]): void
  (e: 'sendToRepeater', resultId: string): void
  (e: 'sendToComparer', resultId: string): void
}>()

const { t } = useI18n()
const { enabledTargets } = useTrafficSendTargets()
const activeView = ref<'results' | 'positions'>('results')
const resultContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  resultId: null as string | null,
})

const captureSummary = computed(() => summarizeIntruderResultFilter(props.captureFilter, 'capture'))
const viewSummary = computed(() => summarizeIntruderResultFilter(props.viewFilter, 'view'))
const allColumns = computed(() =>
  getIntruderResultColumns(props.grepMatchRules, props.grepExtractRules, props.grepPayloadSettings).map((column) => ({
    ...column,
    label: getColumnLabel(column.key, column.label),
  })),
)
const visibleColumnKeys = computed(() => {
  const availableKeys = allColumns.value.map((column) => column.key)
  const selectedKeys = props.visibleColumns.filter((key) => availableKeys.includes(key))
  return selectedKeys.length ? selectedKeys : availableKeys
})
const displayedColumns = computed(() =>
  visibleColumnKeys.value
    .map((key) => allColumns.value.find((column) => column.key === key))
    .filter((column): column is NonNullable<typeof column> => Boolean(column)),
)
const chooserColumns = computed(() => {
  const hiddenColumns = allColumns.value.filter((column) => !visibleColumnKeys.value.includes(column.key))
  return [...displayedColumns.value, ...hiddenColumns]
})
const baselineResult = computed(() => props.results.find((result) => result.isBaseline) ?? null)
const visibleResults = computed(() =>
  sortIntruderResults(
    props.results.filter((result) => matchesIntruderResultFilter(result, props.viewFilter)),
    props.sort,
  ),
)
const diffSummary = computed(() => buildIntruderDiffSummary(baselineResult.value, props.selectedResult))
const contextMenuResult = computed(() =>
  props.results.find((result) => result.id === resultContextMenu.value.resultId) ?? null,
)
const resultContextSendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['repeater', 'comparer'],
    actions: {
      repeater: contextMenuResult.value?.rawRequest
        ? () => emit('sendToRepeater', contextMenuResult.value!.id)
        : undefined,
      comparer: contextMenuResult.value && !contextMenuResult.value.isBaseline
        ? () => emit('sendToComparer', contextMenuResult.value!.id)
        : undefined,
    },
  }),
)
const resultContextRequestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl: contextMenuResult.value ? copyResultUrl : undefined,
      copyRequest: contextMenuResult.value?.rawRequest ? copyResultRequest : undefined,
      copyAsCurl: contextMenuResult.value ? copyResultCurl : undefined,
      openInBrowser: contextMenuResult.value ? openResultInBrowser : undefined,
    },
  }),
)
const resultContextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: resultContextSendMenuItems.value.map((item) => ({
      ...item,
      onClick: () => handleResultContextMenuAction(item.onClick),
    })),
    requestItems: resultContextRequestActionMenuItems.value.map((item) => ({
      ...item,
      onClick: () => handleResultContextMenuAction(item.onClick),
    })),
  }),
)

function getColumnLabel(key: string, fallbackLabel: string): string {
  switch (key) {
    case 'index':
      return t('trafficAnalysis.intruder.labels.requestNumber')
    case 'payloadSummary':
      return t('trafficAnalysis.intruder.labels.payloads')
    case 'payloadReflectionCount':
      return t('trafficAnalysis.intruder.labels.payloadReflections')
    case 'redirectCount':
      return t('trafficAnalysis.intruder.labels.redirects')
    case 'statusCode':
      return t('trafficAnalysis.intruder.labels.status')
    case 'responseTimeMs':
      return t('trafficAnalysis.intruder.labels.responseReceived')
    case 'error':
      return t('trafficAnalysis.intruder.labels.error')
    case 'responseLength':
      return t('trafficAnalysis.intruder.labels.length')
    default:
      return fallbackLabel
  }
}

function toggleSort(key: string) {
  emit('update:sort', {
    key,
    direction: props.sort.key === key && props.sort.direction === 'asc' ? 'desc' : 'asc',
  })
}

function toggleVisibleColumn(key: string, checked: boolean) {
  const nextVisibleColumns = checked
    ? [...new Set([...visibleColumnKeys.value, key])]
    : visibleColumnKeys.value.filter((columnKey) => columnKey !== key)

  if (!nextVisibleColumns.length) {
    return
  }

  emit('update:visibleColumns', nextVisibleColumns)
}

function selectAllColumns() {
  emit('update:visibleColumns', allColumns.value.map((column) => column.key))
}

function showBaseColumnsOnly() {
  emit('update:visibleColumns', INTRUDER_BASE_RESULT_COLUMNS.map((column) => column.key))
}

function resetVisibleColumns() {
  emit('update:visibleColumns', allColumns.value.map((column) => column.key))
}

function moveColumn(key: string, direction: 'up' | 'down') {
  const index = visibleColumnKeys.value.findIndex((columnKey) => columnKey === key)
  if (index === -1) return

  const targetIndex = direction === 'up' ? index - 1 : index + 1
  if (targetIndex < 0 || targetIndex >= visibleColumnKeys.value.length) return

  const nextVisibleColumns = [...visibleColumnKeys.value]
  const [column] = nextVisibleColumns.splice(index, 1)
  nextVisibleColumns.splice(targetIndex, 0, column)
  emit('update:visibleColumns', nextVisibleColumns)
}

function getVisibleColumnIndex(key: string): number {
  return visibleColumnKeys.value.findIndex((columnKey) => columnKey === key)
}

function renderSortMarker(key: string): string {
  if (props.sort.key !== key) return ''
  return props.sort.direction === 'asc' ? '↑' : '↓'
}

function getColumnDisplayValue(result: IntruderAttackResult, key: string): string {
  if (key === 'payloadSummary' && result.isBaseline) {
    return t('trafficAnalysis.intruder.labels.baseline')
  }
  const value = getIntruderResultColumnValue(result, key)
  if (typeof value === 'boolean') {
    return value ? 'Hit' : '-'
  }
  if (typeof value === 'number' && key.startsWith('match:')) {
    return value > 0 ? String(value) : '-'
  }
  return String(value ?? '')
}

function getColumnBadgeClass(result: IntruderAttackResult, key: string): string {
  return getIntruderResultColumnValue(result, key) ? 'badge-success' : 'badge-ghost'
}

function formatDelta(value: number | null): string {
  if (value == null) return '-'
  return value > 0 ? `+${value}` : String(value)
}

function deltaClass(value: number | null): string {
  if (value == null || value === 0) return 'badge-ghost'
  return value > 0 ? 'badge-warning' : 'badge-success'
}

function hideResultContextMenu() {
  resultContextMenu.value.visible = false
  resultContextMenu.value.resultId = null
  document.removeEventListener('click', hideResultContextMenu)
  document.removeEventListener('contextmenu', hideResultContextMenu)
}

function showResultContextMenu(event: MouseEvent, resultId: string) {
  event.preventDefault()
  event.stopPropagation()
  emit('selectResult', resultId)
  resultContextMenu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - 220),
    y: Math.min(event.clientY, window.innerHeight - 280),
    resultId,
  }
  setTimeout(() => {
    document.addEventListener('click', hideResultContextMenu)
    document.addEventListener('contextmenu', hideResultContextMenu)
  }, 0)
}

function showSelectedResultContextMenu(event: MouseEvent) {
  if (!props.selectedResult) return
  showResultContextMenu(event, props.selectedResult.id)
}

async function copyResultUrl() {
  const result = contextMenuResult.value
  if (!result) return

  const url = resolveIntruderResultRequestUrl(result)
  if (!url) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.invalidResultRequestUrl'))
    return
  }

  try {
    await navigator.clipboard.writeText(url)
    dialog.toast.success(t('trafficAnalysis.intruder.messages.urlCopied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.intruder.messages.copyFailed'))
  }
}

async function copyResultRequest() {
  const result = contextMenuResult.value
  if (!result?.rawRequest) return

  try {
    await navigator.clipboard.writeText(result.rawRequest)
    dialog.toast.success(t('trafficAnalysis.intruder.messages.requestCopied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.intruder.messages.copyFailed'))
  }
}

async function copyResultCurl() {
  const result = contextMenuResult.value
  if (!result) return

  const curl = buildIntruderResultCurlCommand(result)
  if (!curl) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.invalidStoredRequest'))
    return
  }

  try {
    await navigator.clipboard.writeText(curl)
    dialog.toast.success(t('trafficAnalysis.intruder.messages.curlCopied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.intruder.messages.copyFailed'))
  }
}

function openResultInBrowser() {
  const result = contextMenuResult.value
  if (!result) return

  const url = resolveIntruderResultRequestUrl(result)
  if (!url) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.invalidResultRequestUrl'))
    return
  }

  window.open(url, '_blank')
}

function handleResultContextMenuAction(action: () => void | Promise<void>) {
  hideResultContextMenu()
  void action()
}

onUnmounted(() => {
  document.removeEventListener('click', hideResultContextMenu)
  document.removeEventListener('contextmenu', hideResultContextMenu)
})
</script>
