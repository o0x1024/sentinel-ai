<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <div class="flex flex-wrap items-center gap-3 border-b border-base-300 bg-base-200 px-4 py-3">
      <div class="min-w-0 flex-1">
        <h2 class="text-sm font-semibold">{{ $t('trafficAnalysis.oast.title') }}</h2>
        <p class="text-xs text-base-content/60">
          {{ $t('trafficAnalysis.oast.description') }}
        </p>
      </div>
      <button class="btn btn-sm btn-primary" type="button" :disabled="creatingToken" @click="createToken">
        <i :class="creatingToken ? 'fas fa-spinner fa-spin' : 'fas fa-plus'"></i>
        <span>{{ $t('trafficAnalysis.oast.createToken') }}</span>
      </button>
      <button class="btn btn-sm btn-outline" type="button" :disabled="syncing" @click="syncRecords">
        <i :class="syncing ? 'fas fa-spinner fa-spin' : 'fas fa-rotate'"></i>
        <span>{{ $t('trafficAnalysis.oast.sync') }}</span>
      </button>
      <button
        class="btn btn-sm btn-outline"
        type="button"
        :disabled="exporting || filteredRecords.length === 0"
        @click="exportRecords"
      >
        <i :class="exporting ? 'fas fa-spinner fa-spin' : 'fas fa-file-export'"></i>
        <span>{{ $t('trafficAnalysis.oast.export') }}</span>
      </button>
      <button class="btn btn-sm btn-ghost" type="button" @click="$emit('openConfig')">
        <i class="fas fa-cog"></i>
        <span>{{ $t('trafficAnalysis.oast.openConfig') }}</span>
      </button>
    </div>

    <div v-if="loading" class="flex flex-1 items-center justify-center">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <div v-else-if="!config.enabled" class="flex flex-1 items-center justify-center p-6">
      <div class="max-w-lg rounded-2xl border border-warning/30 bg-warning/10 p-5 text-center">
        <p class="text-sm font-medium">{{ $t('trafficAnalysis.oast.disabledTitle') }}</p>
        <p class="mt-2 text-xs text-base-content/70">{{ $t('trafficAnalysis.oast.disabledDesc') }}</p>
        <button class="btn btn-sm btn-primary mt-4" type="button" @click="$emit('openConfig')">
          {{ $t('trafficAnalysis.oast.openConfig') }}
        </button>
      </div>
    </div>

    <div v-else class="min-h-0 flex-1 overflow-auto p-4">
      <div v-if="records.length === 0" class="rounded-2xl border border-dashed border-base-300 p-8 text-center">
        <i class="fas fa-satellite-dish text-3xl text-base-content/30"></i>
        <p class="mt-3 text-sm font-medium">{{ $t('trafficAnalysis.oast.emptyTitle') }}</p>
        <p class="mt-1 text-xs text-base-content/60">{{ $t('trafficAnalysis.oast.emptyDesc') }}</p>
      </div>

      <div v-else class="space-y-4">
        <div class="flex flex-wrap items-center gap-3 rounded-2xl border border-base-300 bg-base-200/40 px-4 py-3">
          <label class="min-w-0 flex-1">
            <span class="sr-only">{{ $t('trafficAnalysis.oast.searchPlaceholder') }}</span>
            <input
              v-model.trim="searchQuery"
              type="text"
              class="input input-bordered input-sm w-full"
              :placeholder="$t('trafficAnalysis.oast.searchPlaceholder')"
            />
          </label>
          <label class="min-w-0 flex-1">
            <span class="sr-only">{{ $t('trafficAnalysis.oast.eventSearchPlaceholder') }}</span>
            <input
              v-model.trim="eventSearchQuery"
              type="text"
              class="input input-bordered input-sm w-full"
              :placeholder="$t('trafficAnalysis.oast.eventSearchPlaceholder')"
            />
          </label>
          <div class="join">
            <button
              type="button"
              class="btn btn-sm join-item"
              :class="hitFilter === 'all' ? 'btn-primary' : 'btn-outline'"
              @click="hitFilter = 'all'"
            >
              {{ $t('trafficAnalysis.oast.filterAll') }}
            </button>
            <button
              type="button"
              class="btn btn-sm join-item"
              :class="hitFilter === 'hit' ? 'btn-primary' : 'btn-outline'"
              @click="hitFilter = 'hit'"
            >
              {{ $t('trafficAnalysis.oast.filterHit') }}
            </button>
            <button
              type="button"
              class="btn btn-sm join-item"
              :class="hitFilter === 'pending' ? 'btn-primary' : 'btn-outline'"
              @click="hitFilter = 'pending'"
            >
              {{ $t('trafficAnalysis.oast.filterPending') }}
            </button>
          </div>
          <button
            type="button"
            class="btn btn-sm btn-outline"
            @click="eventSortOrder = eventSortOrder === 'desc' ? 'asc' : 'desc'"
          >
            <i :class="eventSortOrder === 'desc' ? 'fas fa-arrow-down-wide-short' : 'fas fa-arrow-up-short-wide'"></i>
            <span>{{ eventSortOrder === 'desc' ? $t('trafficAnalysis.oast.sortNewest') : $t('trafficAnalysis.oast.sortOldest') }}</span>
          </button>
          <span class="text-xs text-base-content/60">
            {{ $t('trafficAnalysis.oast.filteredCount', { count: filteredRecords.length, total: records.length }) }}
          </span>
        </div>

        <section
          v-if="deleteActivityLog.length"
          class="rounded-2xl border border-base-300 bg-base-200/30 px-4 py-3"
        >
          <div class="flex items-center justify-between gap-3">
            <div>
              <h3 class="text-sm font-medium">{{ $t('trafficAnalysis.oast.deleteActivityTitle') }}</h3>
              <p class="text-xs text-base-content/60">
                {{ $t('trafficAnalysis.oast.deleteActivityDesc') }}
              </p>
            </div>
            <button class="btn btn-xs btn-ghost" type="button" @click="clearDeleteActivityLog">
              {{ $t('trafficAnalysis.oast.clearDeleteActivity') }}
            </button>
          </div>
          <div class="mt-3 space-y-2">
            <div
              v-for="entry in deleteActivityLog"
              :key="entry.id"
              class="rounded-xl border border-base-300 bg-base-100/80 px-3 py-2 text-xs"
            >
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex min-w-0 flex-wrap items-center gap-2">
                  <span class="badge badge-outline">{{ entry.action }}</span>
                  <span class="font-mono text-base-content/70">{{ entry.token }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <span class="text-base-content/50">{{ formatDateTime(entry.at) }}</span>
                  <button class="btn btn-ghost btn-xs" type="button" @click="copyDeleteActivitySummary(entry)">
                    {{ $t('trafficAnalysis.oast.copyDeleteActivity') }}
                  </button>
                </div>
              </div>
              <p class="mt-1 text-base-content/70">{{ entry.summary }}</p>
            </div>
          </div>
        </section>

        <div v-if="filteredRecords.length === 0" class="rounded-2xl border border-dashed border-base-300 p-8 text-center">
          <i class="fas fa-filter text-3xl text-base-content/30"></i>
          <p class="mt-3 text-sm font-medium">{{ $t('trafficAnalysis.oast.noFilteredResultsTitle') }}</p>
          <p class="mt-1 text-xs text-base-content/60">{{ $t('trafficAnalysis.oast.noFilteredResultsDesc') }}</p>
        </div>

        <article
          v-for="record in filteredRecords"
          :key="record.token"
          class="rounded-2xl border border-base-300 bg-base-100 shadow-sm"
        >
          <div class="flex flex-wrap items-start gap-3 border-b border-base-300 bg-base-200/70 px-4 py-3">
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <h3 class="truncate text-sm font-semibold">{{ record.label || record.token }}</h3>
                <span class="badge badge-outline">{{ record.sourceTool }}</span>
                <span class="badge" :class="record.hitCount > 0 ? 'badge-success' : 'badge-ghost'">
                  {{ $t('trafficAnalysis.oast.hitCount', { count: record.hitCount }) }}
                </span>
              </div>
              <p class="mt-1 break-all font-mono text-xs text-base-content/70">{{ record.fqdn }}</p>
              <p v-if="buildSourceRequestSummary(record)" class="mt-1 text-xs text-base-content/65">
                {{ $t('trafficAnalysis.oast.sourceRequest') }}:
                {{ buildSourceRequestSummary(record) }}
              </p>
              <div v-if="getSourceRequest(record)" class="mt-2 flex flex-wrap items-center gap-2 text-[11px]">
                <span class="badge badge-outline">{{ getSourceRequest(record)?.method }}</span>
                <span class="badge badge-ghost">{{ getSourceRequest(record)?.host }}</span>
                <span class="badge" :class="getSourceRequest(record)!.status_code > 0 ? 'badge-success' : 'badge-warning'">
                  {{ getSourceRequest(record)?.status_code || '-' }}
                </span>
                <span class="text-base-content/50">{{ formatDateTime(getSourceRequest(record)?.timestamp || '') }}</span>
              </div>
              <p class="mt-1 text-[11px] text-base-content/50">
                {{ $t('trafficAnalysis.oast.createdAt') }}: {{ formatDateTime(record.createdAt) }}
                <span v-if="record.lastHitAt">
                  · {{ $t('trafficAnalysis.oast.lastHitAt') }}: {{ formatDateTime(record.lastHitAt) }}
                </span>
              </p>
            </div>
            <div class="flex flex-wrap gap-2">
              <button
                v-if="record.sourceRequestId"
                class="btn btn-xs btn-ghost"
                type="button"
                :disabled="openingSourceRequestId === record.sourceRequestId"
                @click="openSourceRequest(record)"
              >
                <i :class="openingSourceRequestId === record.sourceRequestId ? 'fas fa-spinner fa-spin' : 'fas fa-arrow-up-right-from-square'"></i>
                <span>{{ $t('trafficAnalysis.oast.openSourceRequest') }}</span>
              </button>
              <button class="btn btn-xs btn-outline" type="button" @click="copyText(record.httpUrl)">
                HTTP
              </button>
              <button class="btn btn-xs btn-outline" type="button" @click="copyText(record.httpsUrl)">
                HTTPS
              </button>
              <button class="btn btn-xs btn-ghost text-error" type="button" @click="removeRecord(record.token)">
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>

          <div class="grid gap-4 px-4 py-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
            <section class="space-y-2 text-xs">
              <div class="rounded-xl border border-base-300 bg-base-200/40 p-3">
                <div class="font-semibold">{{ $t('trafficAnalysis.oast.payloads') }}</div>
                <div class="mt-2 space-y-2">
                  <div>
                    <div class="text-base-content/60">HTTP</div>
                    <code class="block break-all font-mono">{{ record.httpUrl }}</code>
                  </div>
                  <div>
                    <div class="text-base-content/60">HTTPS</div>
                    <code class="block break-all font-mono">{{ record.httpsUrl }}</code>
                  </div>
                </div>
              </div>
            </section>

            <section class="space-y-3">
              <div class="flex items-center justify-between">
                <div>
                  <h4 class="text-sm font-medium">{{ $t('trafficAnalysis.oast.events') }}</h4>
                  <p class="text-xs text-base-content/60">
                    {{ $t('trafficAnalysis.oast.eventCount', { count: buildFilteredEvents(record).length }) }}
                    <span v-if="getSelectedEventKeySet(record.token).size">
                      · {{ $t('trafficAnalysis.oast.selectedEventCount', { count: getSelectedEventKeySet(record.token).size }) }}
                    </span>
                  </p>
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  <label class="flex items-center gap-2 text-xs text-base-content/70">
                    <input
                      type="checkbox"
                      class="checkbox checkbox-xs"
                      :checked="buildFilteredEvents(record).length > 0 && getSelectedFilteredEventCount(record) === buildFilteredEvents(record).length"
                      @change="toggleSelectAllFilteredEvents(record, ($event.target as HTMLInputElement).checked)"
                    />
                    <span>{{ $t('trafficAnalysis.oast.selectAllFiltered') }}</span>
                  </label>
                  <button
                    v-if="getSelectedEventKeySet(record.token).size"
                    class="btn btn-xs btn-ghost"
                    type="button"
                    @click="clearSelectedEvents(record.token)"
                  >
                    {{ $t('trafficAnalysis.oast.clearSelection') }}
                  </button>
                  <button
                    class="btn btn-xs btn-outline text-error"
                    type="button"
                    :disabled="getSelectedEventKeySet(record.token).size === 0 || isDeletingSelectedEvents(record)"
                    @click="removeSelectedEvents(record)"
                  >
                    <i :class="isDeletingSelectedEvents(record) ? 'fas fa-spinner fa-spin' : 'fas fa-trash-can'"></i>
                    <span>{{ $t('trafficAnalysis.oast.deleteSelectedEvents') }}</span>
                  </button>
                </div>
              </div>

              <div v-if="buildFilteredEvents(record).length === 0" class="rounded-xl border border-dashed border-base-300 p-4 text-xs text-base-content/50">
                {{ $t('trafficAnalysis.oast.noEvents') }}
              </div>

              <div v-else class="space-y-2">
                <div
                  v-for="event in buildFilteredEvents(record)"
                  :key="`${record.token}-${event.time}-${event.url}`"
                  class="rounded-xl border border-base-300 bg-base-200/30 p-3 text-xs"
                >
                  <div class="flex flex-wrap items-center justify-between gap-2">
                    <div class="flex flex-wrap items-center gap-2">
                      <input
                        type="checkbox"
                        class="checkbox checkbox-xs"
                        :checked="isEventSelected(record.token, event)"
                        @change="toggleEventSelection(record.token, event, ($event.target as HTMLInputElement).checked)"
                      />
                      <span class="badge badge-outline">{{ event.method }}</span>
                      <span class="font-medium">{{ formatDateTime(event.time) }}</span>
                      <span v-if="event.country" class="text-base-content/60">{{ event.country }}</span>
                      <span v-if="event.ip" class="font-mono text-base-content/60">{{ event.ip }}</span>
                    </div>
                    <button
                      class="btn btn-ghost btn-xs text-error"
                      type="button"
                      :disabled="isDeletingEvent(record.token, event)"
                      @click="removeEvent(record, event)"
                    >
                      <i :class="isDeletingEvent(record.token, event) ? 'fas fa-spinner fa-spin' : 'fas fa-trash'"></i>
                      <span>{{ $t('trafficAnalysis.oast.deleteEvent') }}</span>
                    </button>
                  </div>
                  <p class="mt-2 break-all font-mono">{{ event.url }}</p>
                  <p v-if="event.userAgent" class="mt-1 break-all text-base-content/60">
                    UA: {{ event.userAgent }}
                  </p>
                </div>
              </div>
            </section>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useI18n } from 'vue-i18n'
import { getProxyRequest, resolveProxyHistoryRequestIdByDbRequestId } from '@/api/trafficHistory'
import { dialog } from '@/composables/useDialog'
import {
  createTrafficOastToken,
  deleteTrafficOastEvents,
  deleteTrafficOastRecord,
  getTrafficOastConfig,
  listTrafficOastRecords,
  syncTrafficOastRecords,
} from '@/api/trafficOast'
import type {
  TrafficOastConfig,
  TrafficOastEvent,
  TrafficOastEventKey,
  TrafficOastRecord,
} from './proxyConfigurationTypes'
import { createDefaultTrafficOastConfig } from './proxyConfigurationTypes'
import type { ProxyRequest } from './proxyHistoryTypes'

const { t } = useI18n()

const emit = defineEmits<{
  (e: 'openConfig'): void
  (e: 'openSourceRequest', requestId: number): void
}>()

const loading = ref(true)
const syncing = ref(false)
const creatingToken = ref(false)
const openingSourceRequestId = ref<number | null>(null)
const searchQuery = ref('')
const hitFilter = ref<'all' | 'hit' | 'pending'>('all')
const eventSearchQuery = ref('')
const eventSortOrder = ref<'desc' | 'asc'>('desc')
const exporting = ref(false)
const config = ref<TrafficOastConfig>(createDefaultTrafficOastConfig())
const records = ref<TrafficOastRecord[]>([])
const sourceRequests = ref<Record<number, ProxyRequest>>({})
const selectedEventKeys = ref<Record<string, string[]>>({})
const deletingEventOperationKeys = ref<string[]>([])
const deleteActivityLog = ref<Array<{
  id: string
  action: string
  token: string
  summary: string
  at: string
}>>([])
let syncTimer: number | null = null

const autoSyncEnabled = computed(() => config.value.enabled && config.value.pollIntervalSecs >= 5)
const filteredRecords = computed(() => {
  const keyword = searchQuery.value.trim().toLowerCase()

  return records.value.filter((record) => {
    if (hitFilter.value === 'hit' && record.hitCount <= 0) {
      return false
    }
    if (hitFilter.value === 'pending' && record.hitCount > 0) {
      return false
    }
    if (!keyword) {
      return true
    }

    const source = getSourceRequest(record)
    const haystack = [
      record.token,
      record.label,
      record.fqdn,
      record.httpUrl,
      record.httpsUrl,
      record.sourceTool,
      buildSourceRequestSummary(record),
      source?.host,
      source?.url,
      source?.method,
      source?.status_code?.toString(),
    ]
      .filter((value): value is string => typeof value === 'string' && value.length > 0)
      .join('\n')
      .toLowerCase()

    return haystack.includes(keyword)
  })
})

function sortRecords(items: TrafficOastRecord[]) {
  return [...items].sort((left, right) => right.createdAt.localeCompare(left.createdAt))
}

function buildEventKey(event: Pick<TrafficOastEvent, 'time' | 'method' | 'url' | 'ip'>) {
  return [event.time || '', event.method || '', event.url || '', event.ip || ''].join('\n')
}

function buildDeleteEventKey(event: TrafficOastEvent): TrafficOastEventKey {
  return {
    time: event.time || '',
    method: event.method || '',
    url: event.url || '',
    ip: event.ip || '',
  }
}

function buildRecordEventOperationKey(token: string, event: Pick<TrafficOastEvent, 'time' | 'method' | 'url' | 'ip'>) {
  return `${token}::${buildEventKey(event)}`
}

function pruneSelectedEventKeys(items: TrafficOastRecord[]) {
  const next: Record<string, string[]> = {}

  for (const record of items) {
    const selected = selectedEventKeys.value[record.token]
    if (!selected?.length) {
      continue
    }

    const validKeys = new Set(record.events.map(event => buildEventKey(event)))
    const remaining = selected.filter(key => validKeys.has(key))
    if (remaining.length) {
      next[record.token] = remaining
    }
  }

  selectedEventKeys.value = next
  deletingEventOperationKeys.value = deletingEventOperationKeys.value.filter((operationKey) => {
    const [token, ...rest] = operationKey.split('::')
    const eventKey = rest.join('::')
    const record = items.find(item => item.token === token)
    if (!record) {
      return false
    }
    return record.events.some(event => buildEventKey(event) === eventKey)
  })
}

function applyRecords(items: TrafficOastRecord[]) {
  records.value = sortRecords(items)
  pruneSelectedEventKeys(records.value)
}

function pushDeleteActivity(action: string, token: string, summary: string) {
  deleteActivityLog.value = [
    {
      id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
      action,
      token,
      summary,
      at: new Date().toISOString(),
    },
    ...deleteActivityLog.value,
  ].slice(0, 6)
}

function clearDeleteActivityLog() {
  deleteActivityLog.value = []
}

async function copyDeleteActivitySummary(entry: { action: string; token: string; summary: string; at: string }) {
  const content = [
    `${t('trafficAnalysis.oast.deleteActivityActionLabel')}: ${entry.action}`,
    `Token: ${entry.token}`,
    `${t('trafficAnalysis.oast.deleteActivityTimeLabel')}: ${formatDateTime(entry.at)}`,
    `${t('trafficAnalysis.oast.deleteActivitySummaryLabel')}: ${entry.summary}`,
  ].join('\n')

  try {
    await navigator.clipboard.writeText(content)
    dialog.toast.success(t('trafficAnalysis.oast.copiedDeleteActivity'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.oast.copyFailed'))
  }
}

async function loadPanel() {
  loading.value = true
  try {
    const [loadedConfig, loadedRecords] = await Promise.all([
      getTrafficOastConfig(),
      listTrafficOastRecords(),
    ])
    config.value = loadedConfig
    applyRecords(loadedRecords)
    await hydrateSourceRequests(records.value)
    scheduleSync()
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to load panel:', error)
    dialog.toast.error(String(error))
  } finally {
    loading.value = false
  }
}

function clearSyncTimer() {
  if (syncTimer) {
    window.clearInterval(syncTimer)
    syncTimer = null
  }
}

function scheduleSync() {
  clearSyncTimer()
  if (!autoSyncEnabled.value) {
    return
  }
  syncTimer = window.setInterval(() => {
    void syncRecords()
  }, config.value.pollIntervalSecs * 1000)
}

async function createToken() {
  creatingToken.value = true
  try {
    const record = await createTrafficOastToken({
      sourceTool: 'traffic-oast-panel',
    })
    applyRecords([record, ...records.value.filter(item => item.token !== record.token)])
    dialog.toast.success(t('trafficAnalysis.oast.created'))
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to create token:', error)
    dialog.toast.error(String(error))
  } finally {
    creatingToken.value = false
  }
}

async function syncRecords() {
  syncing.value = true
  try {
    applyRecords(await syncTrafficOastRecords())
    await hydrateSourceRequests(records.value)
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to sync records:', error)
    dialog.toast.error(String(error))
  } finally {
    syncing.value = false
  }
}

async function removeRecord(token: string) {
  const confirmed = await dialog.confirm(
    t('trafficAnalysis.oast.confirmDeleteRecord'),
  )
  if (!confirmed) {
    return
  }

  try {
    const result = await deleteTrafficOastRecord(token)
    applyRecords(records.value.filter(record => record.token !== token))
    pruneSourceRequests(records.value)
    const summary = t('trafficAnalysis.oast.deletedRecordSummary', {
      remote: result.remoteDeletedAll
        ? t('trafficAnalysis.oast.remoteDeleted')
        : t('trafficAnalysis.oast.remoteDeleteUnknown'),
      local: result.localRemoved
        ? t('trafficAnalysis.oast.localRemoved')
        : t('trafficAnalysis.oast.localRemoveUnknown'),
    })
    pushDeleteActivity(
      t('trafficAnalysis.oast.deleteRecordAction'),
      token,
      summary,
    )
    dialog.toast.success(summary)
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to delete record:', error)
    dialog.toast.error(String(error))
  }
}

function pruneSourceRequests(items: TrafficOastRecord[]) {
  const activeIds = new Set(
    items
      .map(record => record.sourceRequestId)
      .filter((value): value is number => Number.isFinite(value) && value > 0),
  )

  sourceRequests.value = Object.fromEntries(
    Object.entries(sourceRequests.value).filter(([key]) => activeIds.has(Number(key))),
  )
}

function getSourceRequest(record: TrafficOastRecord) {
  if (!record.sourceRequestId) {
    return null
  }
  return sourceRequests.value[record.sourceRequestId] || null
}

function getSelectedEventKeySet(token: string) {
  return new Set(selectedEventKeys.value[token] || [])
}

function isEventSelected(token: string, event: TrafficOastEvent) {
  return getSelectedEventKeySet(token).has(buildEventKey(event))
}

function toggleEventSelection(token: string, event: TrafficOastEvent, checked: boolean) {
  const eventKey = buildEventKey(event)
  const next = new Set(selectedEventKeys.value[token] || [])
  if (checked) {
    next.add(eventKey)
  } else {
    next.delete(eventKey)
  }

  if (next.size === 0) {
    const cloned = { ...selectedEventKeys.value }
    delete cloned[token]
    selectedEventKeys.value = cloned
    return
  }

  selectedEventKeys.value = {
    ...selectedEventKeys.value,
    [token]: [...next],
  }
}

function clearSelectedEvents(token: string) {
  if (!selectedEventKeys.value[token]?.length) {
    return
  }

  const cloned = { ...selectedEventKeys.value }
  delete cloned[token]
  selectedEventKeys.value = cloned
}

function toggleSelectAllFilteredEvents(record: TrafficOastRecord, checked: boolean) {
  const filteredEvents = buildFilteredEvents(record)
  if (!filteredEvents.length) {
    clearSelectedEvents(record.token)
    return
  }

  if (!checked) {
    clearSelectedEvents(record.token)
    return
  }

  selectedEventKeys.value = {
    ...selectedEventKeys.value,
    [record.token]: filteredEvents.map(event => buildEventKey(event)),
  }
}

function getSelectedEvents(record: TrafficOastRecord) {
  const selected = getSelectedEventKeySet(record.token)
  if (!selected.size) {
    return []
  }
  return record.events.filter(event => selected.has(buildEventKey(event)))
}

function getSelectedFilteredEventCount(record: TrafficOastRecord) {
  const selected = getSelectedEventKeySet(record.token)
  if (!selected.size) {
    return 0
  }
  return buildFilteredEvents(record).filter(event => selected.has(buildEventKey(event))).length
}

function isDeletingEvent(token: string, event: TrafficOastEvent) {
  return deletingEventOperationKeys.value.includes(buildRecordEventOperationKey(token, event))
}

function isDeletingSelectedEvents(record: TrafficOastRecord) {
  return getSelectedEvents(record).some(event => isDeletingEvent(record.token, event))
}

function upsertRecord(updated: TrafficOastRecord) {
  applyRecords(records.value.map(record => record.token === updated.token ? updated : record))
}

async function removeEvent(record: TrafficOastRecord, event: TrafficOastEvent) {
  const confirmed = await dialog.confirm(
    t('trafficAnalysis.oast.confirmDeleteEvent'),
  )
  if (!confirmed) {
    return
  }

  const operationKey = buildRecordEventOperationKey(record.token, event)
  deletingEventOperationKeys.value = [...deletingEventOperationKeys.value, operationKey]

  try {
    const result = await deleteTrafficOastEvents(record.token, [buildDeleteEventKey(event)])
    upsertRecord(result.record)
    const summary = t('trafficAnalysis.oast.deletedEventsSummary', {
      deleted: result.deletedCount,
      remaining: result.remainingEventCount,
    })
    pushDeleteActivity(
      t('trafficAnalysis.oast.deleteEventAction'),
      record.token,
      summary,
    )
    dialog.toast.success(summary)
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to delete OAST event:', error)
    dialog.toast.error(String(error))
  } finally {
    deletingEventOperationKeys.value = deletingEventOperationKeys.value.filter(key => key !== operationKey)
  }
}

async function removeSelectedEvents(record: TrafficOastRecord) {
  const selectedEvents = getSelectedEvents(record)
  if (!selectedEvents.length) {
    return
  }

  const confirmed = await dialog.confirm(
    t('trafficAnalysis.oast.confirmDeleteSelectedEvents', { count: selectedEvents.length }),
  )
  if (!confirmed) {
    return
  }

  const operationKeys = selectedEvents.map(event => buildRecordEventOperationKey(record.token, event))
  deletingEventOperationKeys.value = [...new Set([...deletingEventOperationKeys.value, ...operationKeys])]

  try {
    const result = await deleteTrafficOastEvents(
      record.token,
      selectedEvents.map(event => buildDeleteEventKey(event)),
    )
    clearSelectedEvents(record.token)
    upsertRecord(result.record)
    const summary = t('trafficAnalysis.oast.deletedEventsSummary', {
      deleted: result.deletedCount,
      remaining: result.remainingEventCount,
    })
    pushDeleteActivity(
      t('trafficAnalysis.oast.deleteSelectedEventsAction', { count: selectedEvents.length }),
      record.token,
      summary,
    )
    dialog.toast.success(summary)
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to delete selected OAST events:', error)
    dialog.toast.error(String(error))
  } finally {
    deletingEventOperationKeys.value = deletingEventOperationKeys.value.filter(
      key => !operationKeys.includes(key),
    )
  }
}

function buildFilteredEvents(record: TrafficOastRecord) {
  const keyword = eventSearchQuery.value.trim().toLowerCase()
  const filtered = record.events.filter((event) => {
    if (!keyword) {
      return true
    }

    const haystack = [
      event.method,
      event.url,
      event.host,
      event.path,
      event.ip,
      event.userAgent,
      event.country,
      event.referer,
      event.ray,
      event.colo,
      JSON.stringify(event.query || {}),
    ]
      .filter((value): value is string => typeof value === 'string' && value.length > 0)
      .join('\n')
      .toLowerCase()

    return haystack.includes(keyword)
  })

  return [...filtered].sort((left, right) => {
    const leftTime = Date.parse(left.time || '')
    const rightTime = Date.parse(right.time || '')
    const normalizedLeft = Number.isNaN(leftTime) ? 0 : leftTime
    const normalizedRight = Number.isNaN(rightTime) ? 0 : rightTime
    return eventSortOrder.value === 'desc'
      ? normalizedRight - normalizedLeft
      : normalizedLeft - normalizedRight
  })
}

async function hydrateSourceRequests(items: TrafficOastRecord[]) {
  const sourceRequestIds = [
    ...new Set(
      items
        .map(record => record.sourceRequestId)
        .filter((value): value is number => Number.isFinite(value) && value > 0),
    ),
  ]

  pruneSourceRequests(items)

  const missingIds = sourceRequestIds.filter(id => !sourceRequests.value[id])
  if (!missingIds.length) {
    return
  }

  const loadedEntries = await Promise.all(
    missingIds.map(async (dbRequestId) => {
      try {
        const historyRequestId = await resolveProxyHistoryRequestIdByDbRequestId(dbRequestId)
        if (!historyRequestId) {
          return null
        }
        const request = await getProxyRequest(historyRequestId)
        if (!request) {
          return null
        }
        return [dbRequestId, request] as const
      } catch (error) {
        console.error(`[TrafficOastPanel] Failed to load source request #${dbRequestId}:`, error)
        return null
      }
    }),
  )

  const next = { ...sourceRequests.value }
  for (const entry of loadedEntries) {
    if (!entry) continue
    const [dbRequestId, request] = entry
    next[dbRequestId] = request
  }
  sourceRequests.value = next
}

function buildSourceRequestSummary(record: TrafficOastRecord) {
  if (!record.sourceRequestId) {
    return ''
  }

  const request = getSourceRequest(record)
  if (!request) {
    return `#${record.sourceRequestId}`
  }

  return `${request.method} ${request.url}`
}

function buildExportFilename() {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
  return `traffic-oast-${timestamp}.json`
}

async function exportRecords() {
  exporting.value = true
  try {
    const selected = await save({
      defaultPath: buildExportFilename(),
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })

    if (!selected) {
      return
    }

    const payload = filteredRecords.value.map((record) => ({
      ...record,
      sourceRequest: getSourceRequest(record),
      filteredEvents: buildFilteredEvents(record),
    }))

    await writeTextFile(selected, JSON.stringify(payload, null, 2))
    dialog.toast.success(t('trafficAnalysis.oast.exported'))
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to export records:', error)
    dialog.toast.error(String(error))
  } finally {
    exporting.value = false
  }
}

async function openSourceRequest(record: TrafficOastRecord) {
  if (!record.sourceRequestId) {
    dialog.toast.warning(t('trafficAnalysis.oast.sourceRequestUnavailable'))
    return
  }

  openingSourceRequestId.value = record.sourceRequestId
  try {
    const historyRequestId = await resolveProxyHistoryRequestIdByDbRequestId(record.sourceRequestId)
    if (!historyRequestId) {
      dialog.toast.warning(t('trafficAnalysis.oast.sourceRequestUnavailable'))
      return
    }

    dialog.toast.success(t('trafficAnalysis.oast.openedSourceRequest'))
    emit('openSourceRequest', historyRequestId)
  } catch (error) {
    console.error('[TrafficOastPanel] Failed to open source request:', error)
    dialog.toast.error(String(error))
  } finally {
    openingSourceRequestId.value = null
  }
}

async function copyText(value: string) {
  try {
    await navigator.clipboard.writeText(value)
    dialog.toast.success(t('trafficAnalysis.oast.copied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.oast.copyFailed'))
  }
}

function formatDateTime(value: string) {
  if (!value) {
    return '-'
  }
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString()
}

onMounted(() => {
  void loadPanel()
})

onUnmounted(() => {
  clearSyncTimer()
})
</script>
