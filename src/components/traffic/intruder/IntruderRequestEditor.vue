<template>
  <section class="flex h-full min-h-0 flex-col rounded-l-lg border border-base-300 bg-base-100">
    <div class="border-b border-base-300 px-4 py-3">
      <div class="flex flex-wrap items-center gap-3">
        <label class="flex min-w-0 flex-1 items-center gap-3">
          <span class="w-14 text-sm text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.target') }}</span>
          <input
            :value="targetUrl"
            type="text"
            class="input input-bordered input-sm flex-1"
            :placeholder="$t('trafficAnalysis.intruder.placeholders.targetUrl')"
            @input="$emit('update:targetUrl', ($event.target as HTMLInputElement).value)"
          />
        </label>

        <label class="flex items-center gap-2 text-sm">
          <input
            :checked="updateHostHeader"
            type="checkbox"
            class="checkbox checkbox-sm checkbox-primary"
            @change="$emit('update:updateHostHeader', ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.updateHostHeader') }}</span>
        </label>
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-2 border-b border-base-300 bg-base-200 px-4 py-2">
      <span class="text-sm font-medium text-base-content/70">{{ $t('trafficAnalysis.intruder.sections.positions') }}</span>
      <button class="btn btn-sm btn-ghost" type="button" @click="markSelection">
        {{ $t('trafficAnalysis.intruder.actions.addPositionSymbol') }}
      </button>
      <button class="btn btn-sm btn-ghost" type="button" @click="$emit('clearMarkers')">
        {{ $t('trafficAnalysis.intruder.actions.clearPositionSymbol') }}
      </button>
      <button class="btn btn-sm btn-ghost" type="button" @click="$emit('autoMark')">
        {{ $t('trafficAnalysis.intruder.actions.autoMark') }}
      </button>
      <div class="ml-auto flex items-center gap-3 text-xs text-base-content/70">
        <span>{{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.detectedPositions') }}</span>
        <span>{{ requestLengthLabel }}</span>
      </div>
    </div>

    <div class="min-h-0 flex-1" @contextmenu.capture.prevent="showContextMenu($event)">
      <HttpMessageSurface
        ref="requestEditor"
        :model-value="requestText"
        custom-context-menu
        show-search-bar
        message-type="request"
        marker-mode="intruder"
        height="100%"
        display-mode="raw"
        :state-key="`intruder:editor:${targetUrl || 'default'}`"
        :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
        :search-next-title="$t('trafficAnalysis.messageSearch.next')"
        :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
        :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
        :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
        :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
        :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
        :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
        @update:model-value="$emit('update:requestText', $event)"
        @contextmenu="showContextMenu($event)"
      />
    </div>

    <div
      v-if="contextMenu.visible"
      class="fixed z-50 min-w-48 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
      :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="contextMenuSections"
        label-prefix="trafficAnalysis.intruder.contextMenu"
      />
    </div>

    <div class="flex items-center gap-3 border-t border-base-300 bg-base-200 px-4 py-2 text-xs text-base-content/70">
      <span>{{ $t('trafficAnalysis.intruder.help.markerHint') }}</span>
      <span class="ml-auto">{{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.positionCount') }}</span>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import TrafficContextMenuSections from '@/components/traffic/TrafficContextMenuSections.vue'
import { buildTrafficRequestActionMenuItems } from '@/components/traffic/trafficRequestActionMenuSupport'
import { buildTrafficRequestContextMenuSections } from '@/components/traffic/trafficRequestContextMenuSupport'
import { buildTrafficRequestSendMenuItems } from '@/components/traffic/trafficSendMenuSupport'
import { useTrafficSendTargets } from '@/components/traffic/trafficSendTargets'
import type { IntruderPosition } from './types'
import { buildFullUrl, buildSourceRequestFromRawRequest, extractTargetFromRequest } from './http'
import { wrapSelectionWithMarkers } from './intruderMarkers'

const { t } = useI18n()
const { enabledTargets } = useTrafficSendTargets()

const props = defineProps<{
  requestText: string
  targetUrl: string
  updateHostHeader: boolean
  positions: IntruderPosition[]
}>()

const emit = defineEmits<{
  (e: 'update:requestText', value: string): void
  (e: 'update:targetUrl', value: string): void
  (e: 'update:updateHostHeader', value: boolean): void
  (e: 'autoMark'): void
  (e: 'clearMarkers'): void
  (e: 'sendToRepeater'): void
  (e: 'sendDraftRequestToComparer'): void
}>()

const requestEditor = ref<InstanceType<typeof HttpMessageSurface> | null>(null)
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
})

const requestLengthLabel = computed(() => {
  const length = props.requestText.length
  return `${t('trafficAnalysis.intruder.labels.length')}: ${length}`
})
const currentTarget = computed(() => extractTargetFromRequest(props.requestText, props.targetUrl))
const currentRequest = computed(() => buildSourceRequestFromRawRequest(props.requestText, currentTarget.value))
const currentUrl = computed(() => buildFullUrl(props.requestText, currentTarget.value))
const sendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['repeater', 'comparer'],
    actions: {
      repeater: currentRequest.value ? () => emit('sendToRepeater') : undefined,
      comparer: currentRequest.value ? () => emit('sendDraftRequestToComparer') : undefined,
    },
  }),
)
const requestActionMenuItems = computed(() =>
  buildTrafficRequestActionMenuItems({
    supportedActions: ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser'],
    actions: {
      copyUrl: currentUrl.value ? copyUrl : undefined,
      copyRequest: props.requestText ? copyRequest : undefined,
      copyAsCurl: currentRequest.value ? copyAsCurl : undefined,
      openInBrowser: currentUrl.value ? openInBrowser : undefined,
    },
  }),
)
const contextMenuSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: sendMenuItems.value.map((item) => ({
      ...item,
      onClick: () => handleContextMenuAction(item.onClick),
    })),
    requestItems: requestActionMenuItems.value.map((item) => ({
      ...item,
      onClick: () => handleContextMenuAction(item.onClick),
    })),
  }),
)

async function markSelection() {
  const selection = requestEditor.value?.getSelectionRange()
  if (!selection || selection.from === selection.to) {
    dialog.toast.info(t('trafficAnalysis.intruder.messages.selectTextFirst'))
    return
  }

  const start = Math.min(selection.from, selection.to)
  const end = Math.max(selection.from, selection.to)
  const wrapped = wrapSelectionWithMarkers(props.requestText, start, end)
  emit('update:requestText', wrapped)

  await nextTick()
  requestAnimationFrame(() => {
    requestEditor.value?.setSelection?.(start + 1, end + 1)
    requestEditor.value?.focus?.()
  })
}

function hideContextMenu() {
  contextMenu.value.visible = false
  document.removeEventListener('click', hideContextMenu)
  document.removeEventListener('contextmenu', hideContextMenu)
}

function showContextMenu(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  contextMenu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - 220),
    y: Math.min(event.clientY, window.innerHeight - 240),
  }
  setTimeout(() => {
    document.addEventListener('click', hideContextMenu)
    document.addEventListener('contextmenu', hideContextMenu)
  }, 0)
}

function handleContextMenuAction(action: () => void | Promise<void>) {
  hideContextMenu()
  void action()
}

async function copyUrl() {
  if (!currentUrl.value) return
  try {
    await navigator.clipboard.writeText(currentUrl.value)
    dialog.toast.success(t('trafficAnalysis.intruder.messages.urlCopied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.intruder.messages.copyFailed'))
  }
}

async function copyRequest() {
  if (!props.requestText) return
  try {
    await navigator.clipboard.writeText(props.requestText)
    dialog.toast.success(t('trafficAnalysis.intruder.messages.requestCopied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.intruder.messages.copyFailed'))
  }
}

async function copyAsCurl() {
  const request = currentRequest.value
  if (!request) return

  const parts = [`curl -X ${request.request.method}`]
  for (const header of request.request.headers) {
    parts.push(`-H ${quoteForShell(`${header.name}: ${header.value}`)}`)
  }
  if (request.request.bodyText) {
    parts.push(`--data-raw ${quoteForShell(request.request.bodyText)}`)
  }
  parts.push(quoteForShell(request.absoluteUrl))

  try {
    await navigator.clipboard.writeText(parts.join(' '))
    dialog.toast.success(t('trafficAnalysis.intruder.messages.curlCopied'))
  } catch {
    dialog.toast.error(t('trafficAnalysis.intruder.messages.copyFailed'))
  }
}

function openInBrowser() {
  if (!currentUrl.value) return
  window.open(currentUrl.value, '_blank')
}

function quoteForShell(value: string) {
  return `'${value.replace(/'/g, `'\"'\"'`)}'`
}

onUnmounted(() => {
  document.removeEventListener('click', hideContextMenu)
  document.removeEventListener('contextmenu', hideContextMenu)
})
</script>
