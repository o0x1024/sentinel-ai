<template>
  <section class="flex h-full min-h-0 flex-col rounded-l-lg border border-base-300 bg-base-100">
    <div class="border-b border-base-300 px-3 py-2">
      <div class="flex flex-wrap items-center gap-2">
        <label class="flex min-w-0 flex-1 items-center gap-2">
          <span class="w-12 text-sm text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.target') }}</span>
          <input
            :value="targetUrl"
            type="text"
            class="input input-bordered input-sm flex-1"
            :placeholder="$t('trafficAnalysis.intruder.placeholders.targetUrl')"
            @input="$emit('update:targetUrl', ($event.target as HTMLInputElement).value)"
          />
        </label>

        <label class="flex items-center gap-1.5 text-sm">
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

    <div class="flex flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 px-3 py-1.5">
      <span class="text-sm font-medium text-base-content/70">{{ $t('trafficAnalysis.intruder.sections.positions') }}</span>
      <div class="ml-1.5 inline-flex items-center gap-1 rounded-full border border-base-300/80 bg-base-100 p-0.5">
        <button
          type="button"
          class="btn btn-ghost btn-xs min-h-6 rounded-full px-2.5"
          :class="props.requestViewTab === 'pretty' ? 'btn-active' : ''"
          @click="$emit('update:requestViewTab', 'pretty')"
        >
          Pretty
        </button>
        <button
          type="button"
          class="btn btn-ghost btn-xs min-h-6 rounded-full px-2.5"
          :class="props.requestViewTab === 'raw' ? 'btn-active' : ''"
          @click="$emit('update:requestViewTab', 'raw')"
        >
          Raw
        </button>
      </div>
      <button class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" @click="markSelection">
        {{ $t('trafficAnalysis.intruder.actions.addPositionSymbol') }}
      </button>
      <button class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" @click="$emit('clearMarkers')">
        {{ $t('trafficAnalysis.intruder.actions.clearPositionSymbol') }}
      </button>
      <button class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" @click="$emit('autoMark')">
        {{ $t('trafficAnalysis.intruder.actions.autoMark') }}
      </button>
      <button class="btn btn-sm btn-ghost min-h-8 px-2.5" type="button" :disabled="creatingOastPayload" @click="insertOastPayload">
        <i :class="creatingOastPayload ? 'fas fa-spinner fa-spin' : 'fas fa-satellite-dish'"></i>
        {{ $t('trafficAnalysis.oast.insertPayload') }}
      </button>
      <div class="ml-auto flex items-center gap-2 text-xs text-base-content/70">
        <span>{{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.detectedPositions') }}</span>
        <span>{{ requestLengthLabel }}</span>
      </div>
    </div>

    <div class="min-h-0 flex-1" @contextmenu.capture.prevent="showContextMenu($event)">
      <HttpMessageSurface
        ref="requestEditor"
        :model-value="displayRequestText"
        custom-context-menu
        show-search-bar
        message-type="request"
        marker-mode="intruder"
        height="100%"
        :display-mode="props.requestViewTab"
        :state-key="`intruder:editor:${targetUrl || 'default'}:${props.requestViewTab}`"
        :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
        :search-next-title="$t('trafficAnalysis.messageSearch.next')"
        :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
        :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
        :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
        :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
        :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
        :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
        @update:model-value="handleRequestEditorUpdate"
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
        :sections="contextMenuBeforeCodecSections"
        label-prefix="trafficAnalysis.intruder.contextMenu"
      />
      <TrafficContextSubmenu
        v-if="textCodecSubmenu"
        :submenu="textCodecSubmenu"
        label-prefix="trafficAnalysis.intruder.contextMenu"
      />
      <div v-if="textCodecSubmenu && contextMenuAfterCodecSections.length" class="divider my-1 h-0"></div>
      <TrafficContextMenuSections
        :sections="contextMenuAfterCodecSections"
        label-prefix="trafficAnalysis.intruder.contextMenu"
      />
    </div>

    <div class="flex items-center gap-2 border-t border-base-300 bg-base-200 px-3 py-1.5 text-xs text-base-content/70">
      <span>{{ $t('trafficAnalysis.intruder.help.markerHint') }}</span>
      <span class="ml-auto">{{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.positionCount') }}</span>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import { createTrafficOastToken } from '@/api/trafficOast'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import TrafficContextMenuSections from '@/components/traffic/TrafficContextMenuSections.vue'
import TrafficContextSubmenu from '@/components/traffic/TrafficContextSubmenu.vue'
import { buildTrafficRequestActionMenuItems } from '@/components/traffic/trafficRequestActionMenuSupport'
import { buildTrafficRequestContextMenuSections } from '@/components/traffic/trafficRequestContextMenuSupport'
import { buildTrafficRequestSendMenuItems } from '@/components/traffic/trafficSendMenuSupport'
import { useTrafficSendTargets } from '@/components/traffic/trafficSendTargets'
import { buildTrafficTextCodecSubmenu, getTrafficTextCodecErrorMessage, hasNonEmptyTextSelection, replaceTrafficTextSelection, transformTrafficTextCodec, type TrafficTextCodecAction } from '@/components/traffic/trafficTextCodecSupport'
import { convertRepeaterPrettyRequestToRaw, formatRepeaterPrettyRequest } from '@/components/traffic/trafficRepeaterPrettyRequestSupport'
import { useTrafficCodec } from '../codec/useTrafficCodec'
import type { IntruderPosition, IntruderRequestViewTab } from './types'
import { buildFullUrl, buildSourceRequestFromRawRequest, extractTargetFromRequest } from './http'
import { decodeIntruderRequestTemplate, type IntruderCodecSession } from './intruderCodecSupport'
import { wrapSelectionWithMarkers } from './intruderMarkers'

const { t } = useI18n()
const { enabledTargets } = useTrafficSendTargets()

const props = defineProps<{
  workspaceId: string
  requestLoadGeneration: number
  requestText: string
  requestViewTab: IntruderRequestViewTab
  targetUrl: string
  sourceRequestId: number | null
  updateHostHeader: boolean
  positions: IntruderPosition[]
}>()

const emit = defineEmits<{
  (e: 'update:requestText', value: string): void
  (e: 'update:requestViewTab', value: IntruderRequestViewTab): void
  (e: 'update:targetUrl', value: string): void
  (e: 'update:updateHostHeader', value: boolean): void
  (e: 'codecSessionChanged', value: IntruderCodecSession | null): void
  (e: 'autoMark'): void
  (e: 'clearMarkers'): void
  (e: 'createDraft'): void
  (e: 'openDraftCompare'): void
}>()

const codec = useTrafficCodec()
const decodedLoadKey = ref('')

const requestEditor = ref<InstanceType<typeof HttpMessageSurface> | null>(null)
const creatingOastPayload = ref(false)
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  selection: null as { from: number; to: number } | null,
})

const requestLengthLabel = computed(() => {
  const length = props.requestText.length
  return `${t('trafficAnalysis.intruder.labels.length')}: ${length}`
})
const displayRequestText = computed(() =>
  props.requestViewTab === 'pretty'
    ? formatRepeaterPrettyRequest(props.requestText)
    : props.requestText,
)
const currentTarget = computed(() => extractTargetFromRequest(props.requestText, props.targetUrl))
const currentRequest = computed(() =>
  buildSourceRequestFromRawRequest(props.requestText, currentTarget.value, props.sourceRequestId),
)
const currentUrl = computed(() => buildFullUrl(props.requestText, currentTarget.value))
const sendMenuItems = computed(() =>
  buildTrafficRequestSendMenuItems({
    enabledTargets: enabledTargets.value,
    supportedTargets: ['draft', 'compare'],
    actions: {
      draft: currentRequest.value ? () => emit('createDraft') : undefined,
      compare: currentRequest.value ? () => emit('openDraftCompare') : undefined,
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
const contextMenuBeforeCodecSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    sendItems: sendMenuItems.value.map((item) => ({
      ...item,
      onClick: () => handleContextMenuAction(item.onClick),
    })),
  }),
)
const contextMenuAfterCodecSections = computed(() =>
  buildTrafficRequestContextMenuSections({
    requestItems: requestActionMenuItems.value.map((item) => ({
      ...item,
      onClick: () => handleContextMenuAction(item.onClick),
    })),
  }),
)
const textCodecSubmenu = computed(() =>
  buildTrafficTextCodecSubmenu({
    disabled: !hasNonEmptyTextSelection(contextMenu.value.selection),
    onClick: action => handleContextMenuAction(() => applyTextCodecToRequestSelection(action)),
  }),
)

function handleRequestEditorUpdate(value: string) {
  emit(
    'update:requestText',
    props.requestViewTab === 'pretty' ? convertRepeaterPrettyRequestToRaw(value) : value,
  )
}

watch(
  () => `${props.workspaceId}:${props.requestLoadGeneration}:${props.sourceRequestId ?? ''}`,
  async (loadKey) => {
    if (loadKey === decodedLoadKey.value) {
      return
    }

    decodedLoadKey.value = loadKey

    if (props.requestLoadGeneration === 0) {
      emit('codecSessionChanged', null)
      return
    }

    const requestText = props.requestText
    if (!requestText.trim()) {
      emit('codecSessionChanged', null)
      return
    }

    const host = extractTargetFromRequest(requestText, props.targetUrl).host
    const { requestText: decodedText, session } = await decodeIntruderRequestTemplate(
      requestText,
      host,
      codec,
    )

    emit('codecSessionChanged', session)

    if (session && decodedText !== requestText) {
      emit('update:requestText', decodedText)
    }
  },
  { immediate: true },
)

function normalizeLineEndings(value: string): string {
  return value.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

function mapNormalizedOffsetToOriginalOffset(content: string, normalizedOffset: number): number {
  if (normalizedOffset <= 0) return 0

  let originalOffset = 0
  let traversed = 0
  while (originalOffset < content.length && traversed < normalizedOffset) {
    if (content[originalOffset] === '\r') {
      originalOffset += 1
      continue
    }

    originalOffset += 1
    traversed += 1
  }

  return originalOffset
}

function scoreContextMatch(rawText: string, candidateStart: number, candidateEnd: number, beforeText: string, afterText: string): number {
  let score = 0

  if (beforeText) {
    const beforeWindow = rawText.slice(Math.max(0, candidateStart - beforeText.length), candidateStart)
    let matched = 0
    while (
      matched < beforeText.length
      && matched < beforeWindow.length
      && beforeWindow[beforeWindow.length - 1 - matched] === beforeText[beforeText.length - 1 - matched]
    ) {
      matched += 1
    }
    score += matched
  }

  if (afterText) {
    const afterWindow = rawText.slice(candidateEnd, candidateEnd + afterText.length)
    let matched = 0
    while (
      matched < afterText.length
      && matched < afterWindow.length
      && afterWindow[matched] === afterText[matched]
    ) {
      matched += 1
    }
    score += matched
  }

  return score
}

function resolvePrettySelectionInRaw(selection: { from: number; to: number }) {
  const prettyText = displayRequestText.value
  const rawTextNormalized = normalizeLineEndings(props.requestText)
  const start = Math.min(selection.from, selection.to)
  const end = Math.max(selection.from, selection.to)
  const selectedText = prettyText.slice(start, end)

  if (!selectedText) {
    return null
  }

  if (rawTextNormalized.slice(start, end) === selectedText) {
    return {
      from: mapNormalizedOffsetToOriginalOffset(props.requestText, start),
      to: mapNormalizedOffsetToOriginalOffset(props.requestText, end),
    }
  }

  const beforeText = prettyText.slice(Math.max(0, start - 32), start)
  const afterText = prettyText.slice(end, Math.min(prettyText.length, end + 32))
  const candidates: Array<{ from: number; to: number; score: number }> = []

  let searchIndex = rawTextNormalized.indexOf(selectedText)
  while (searchIndex !== -1) {
    const candidateEnd = searchIndex + selectedText.length
    candidates.push({
      from: searchIndex,
      to: candidateEnd,
      score: scoreContextMatch(rawTextNormalized, searchIndex, candidateEnd, beforeText, afterText),
    })
    searchIndex = rawTextNormalized.indexOf(selectedText, searchIndex + 1)
  }

  if (!candidates.length) {
    return null
  }

  candidates.sort((left, right) => right.score - left.score)
  const bestCandidate = candidates[0]

  return {
    from: mapNormalizedOffsetToOriginalOffset(props.requestText, bestCandidate.from),
    to: mapNormalizedOffsetToOriginalOffset(props.requestText, bestCandidate.to),
  }
}

function resolveSelectionForRawOperation(selection: { from: number; to: number }) {
  return props.requestViewTab === 'pretty'
    ? resolvePrettySelectionInRaw(selection)
    : selection
}

async function markSelection() {
  const selection = requestEditor.value?.getSelectionRange()
  if (!selection || selection.from === selection.to) {
    dialog.toast.info(t('trafficAnalysis.intruder.messages.selectTextFirst'))
    return
  }

  const resolvedSelection = resolveSelectionForRawOperation(selection)
  if (!resolvedSelection || resolvedSelection.from === resolvedSelection.to) {
    return
  }

  if (props.requestViewTab === 'pretty') {
    emit('update:requestViewTab', 'raw')
  }

  const start = Math.min(resolvedSelection.from, resolvedSelection.to)
  const end = Math.max(resolvedSelection.from, resolvedSelection.to)
  const wrapped = wrapSelectionWithMarkers(props.requestText, start, end)
  emit('update:requestText', wrapped)

  await nextTick()
  requestAnimationFrame(() => {
    requestEditor.value?.setSelection?.(start + 1, end + 1)
    requestEditor.value?.focus?.()
  })
}

async function insertOastPayload() {
  const selection = requestEditor.value?.getSelectionRange()
  const resolvedSelection = selection ? resolveSelectionForRawOperation(selection) : undefined

  if (props.requestViewTab === 'pretty') {
    emit('update:requestViewTab', 'raw')
  }

  creatingOastPayload.value = true
  try {
    const record = await createTrafficOastToken({
      label: currentTarget.value.host || undefined,
      sourceTool: 'intruder',
      sourceRequestId: props.sourceRequestId,
    })
    const next = replaceTrafficTextSelection(
      props.requestText,
      resolvedSelection,
      record.httpsUrl || record.httpUrl || record.fqdn,
    )
    emit('update:requestText', next.content)
    await nextTick()
    requestEditor.value?.setSelection?.(next.selectionStart, next.selectionEnd)
    requestEditor.value?.focus?.()
    dialog.toast.success(t('trafficAnalysis.oast.inserted'))
  } catch (error) {
    console.error('[IntruderRequestEditor] Failed to insert OAST payload:', error)
    dialog.toast.error(String(error))
  } finally {
    creatingOastPayload.value = false
  }
}

async function applyTextCodecToRequestSelection(action: TrafficTextCodecAction) {
  const selection = contextMenu.value.selection
  if (!hasNonEmptyTextSelection(selection)) {
    dialog.toast.warning(t('trafficAnalysis.textCodec.noSelection'))
    return
  }

  const resolvedSelection = resolveSelectionForRawOperation(selection)
  if (!hasNonEmptyTextSelection(resolvedSelection)) {
    dialog.toast.warning(t('trafficAnalysis.textCodec.noSelection'))
    return
  }

  if (props.requestViewTab === 'pretty') {
    emit('update:requestViewTab', 'raw')
  }

  try {
    const replacement = transformTrafficTextCodec(
      props.requestText.slice(resolvedSelection.from, resolvedSelection.to),
      action,
    )
    const next = replaceTrafficTextSelection(props.requestText, resolvedSelection, replacement)
    emit('update:requestText', next.content)
    await nextTick()
    requestEditor.value?.setSelection?.(next.selectionStart, next.selectionEnd)
    requestEditor.value?.focus?.()
    dialog.toast.success(t('trafficAnalysis.textCodec.applied', {
      action: t(`trafficAnalysis.intruder.contextMenu.${action.labelKey}`),
    }))
  } catch (error) {
    dialog.toast.error(t('trafficAnalysis.textCodec.failed', {
      error: getTrafficTextCodecErrorMessage(error),
    }))
  }
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
    selection: requestEditor.value?.getSelectionRange?.() ?? null,
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
