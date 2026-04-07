<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <div class="flex items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1">
      <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
        <div
          v-for="item in items"
          :key="item.id"
          class="flex items-center gap-2 rounded border border-base-300 px-3 py-1.5 text-sm"
          :class="activeItemId === item.id ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
        >
          <button class="truncate" type="button" :title="item.name" @click="activeItemId = item.id">
            {{ item.name }}
          </button>
          <button class="btn btn-ghost btn-xs btn-circle" type="button" @click="closeItem(item.id)">
            <i class="fas fa-times text-[10px]"></i>
          </button>
        </div>
      </div>
      <button class="btn btn-xs btn-ghost" type="button" @click="openDraftComposer">
        <i class="fas fa-plus"></i>
        {{ $t('trafficAnalysis.comparer.actions.newComparison') }}
      </button>
    </div>

    <div v-if="showDraftComposer" class="flex min-h-0 flex-1 flex-col">
      <div class="border-b border-base-300 bg-base-200 px-3 py-2">
        <div class="flex flex-wrap items-center gap-2">
          <span class="font-semibold text-sm">{{ $t('trafficAnalysis.comparer.draft.title') }}</span>
          <button class="btn btn-ghost btn-xs" type="button" @click="pasteDraftSide('left')">
            <i class="fas fa-paste"></i>
            {{ $t('trafficAnalysis.comparer.actions.pasteLeft') }}
          </button>
          <button class="btn btn-ghost btn-xs" type="button" @click="pasteDraftSide('right')">
            <i class="fas fa-paste"></i>
            {{ $t('trafficAnalysis.comparer.actions.pasteRight') }}
          </button>
          <button class="btn btn-ghost btn-xs" type="button" @click="clearDraft">
            <i class="fas fa-eraser"></i>
            {{ $t('trafficAnalysis.comparer.actions.clearDraft') }}
          </button>
          <button class="btn btn-primary btn-xs" type="button" :disabled="!canCreateDraftComparison" @click="createComparisonFromDraft">
            <i class="fas fa-not-equal"></i>
            {{ $t('trafficAnalysis.comparer.actions.createComparison') }}
          </button>
          <div class="flex-1"></div>
          <button
            v-if="currentItem"
            class="btn btn-ghost btn-xs"
            type="button"
            @click="showDraftComposer = false"
          >
            <i class="fas fa-times"></i>
            {{ $t('trafficAnalysis.comparer.actions.closeDraft') }}
          </button>
        </div>
      </div>

      <div class="grid min-h-0 flex-1 gap-px bg-base-300 md:grid-cols-2">
        <div class="flex min-h-0 flex-col bg-base-100">
          <div class="border-b border-base-300 bg-base-200 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
            {{ draft.leftLabel }}
          </div>
          <div class="min-h-0 flex-1" @contextmenu.capture.prevent="showDraftContextMenu($event, 'left')">
            <HttpMessageSurface
              v-model="draft.leftText"
              custom-context-menu
              :message-type="draftLeftMessageType"
              display-mode="raw"
              height="100%"
              :placeholder="$t('trafficAnalysis.comparer.draft.leftPlaceholder')"
              state-key="comparer:draft:left"
              @contextmenu="showDraftContextMenu($event, 'left')"
            />
          </div>
        </div>

        <div class="flex min-h-0 flex-col bg-base-100">
          <div class="border-b border-base-300 bg-base-200 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
            {{ draft.rightLabel }}
          </div>
          <div class="min-h-0 flex-1" @contextmenu.capture.prevent="showDraftContextMenu($event, 'right')">
            <HttpMessageSurface
              v-model="draft.rightText"
              custom-context-menu
              :message-type="draftRightMessageType"
              display-mode="raw"
              height="100%"
              :placeholder="$t('trafficAnalysis.comparer.draft.rightPlaceholder')"
              state-key="comparer:draft:right"
              @contextmenu="showDraftContextMenu($event, 'right')"
            />
          </div>
        </div>
      </div>
    </div>

    <div
      v-else-if="currentItem"
      class="flex min-h-0 flex-1 flex-col"
      @contextmenu.capture.prevent="showContextMenu($event)"
    >
      <div class="flex flex-wrap items-center gap-2 border-b border-base-300 bg-base-200 px-3 py-2">
        <span class="badge badge-outline">{{ compareSourceLabel }}</span>
        <span class="badge badge-outline">{{ compareKindLabel }}</span>
        <span class="badge badge-outline">{{ compareMessageTypeLabel }}</span>
        <span v-if="pinnedBaseline" class="badge badge-accent badge-outline">
          {{ $t('trafficAnalysis.comparer.badges.pinnedBaseline', { label: pinnedBaselineLabel }) }}
        </span>
        <div class="tabs tabs-boxed tabs-xs bg-base-300">
          <button
            type="button"
            :class="['tab tab-xs', viewMode === 'pretty' ? 'tab-active' : '']"
            @click="viewMode = 'pretty'"
          >
            {{ $t('trafficAnalysis.comparer.tabs.pretty') }}
          </button>
          <button
            type="button"
            :class="['tab tab-xs', viewMode === 'raw' ? 'tab-active' : '']"
            @click="viewMode = 'raw'"
          >
            {{ $t('trafficAnalysis.comparer.tabs.raw') }}
          </button>
        </div>
        <button
          v-for="item in comparerToolbarActionMenuItems"
          :key="`comparer-toolbar-action-${item.key}`"
          class="btn btn-ghost btn-xs"
          type="button"
          :disabled="item.disabled"
          @click="item.onClick"
        >
          <i :class="item.iconClass.replace(' text-secondary', '').replace(' text-info', '').replace(' text-warning', '').replace(' text-primary', '')"></i>
          {{ $t(`trafficAnalysis.comparer.actions.${item.labelKey}`) }}
        </button>
        <div class="flex-1"></div>
        <span class="badge badge-outline">{{ currentItem.leftLabel }}: {{ displayedLeftText.length }}</span>
        <span class="badge badge-outline">{{ currentItem.rightLabel }}: {{ displayedRightText.length }}</span>
        <span class="badge badge-outline">{{ $t('trafficAnalysis.comparer.labels.changedLines') }}: {{ diffSummary.changedLines }}</span>
        <span class="badge badge-outline">{{ $t('trafficAnalysis.comparer.labels.similarity') }}: {{ diffSummary.similarity }}%</span>
      </div>

      <div class="min-h-0 flex-1 p-4" @contextmenu.capture.prevent="showContextMenu($event)">
        <CodeDiffViewer
          :left-text="displayedLeftText"
          :right-text="displayedRightText"
          :message-type="diffMessageType"
          @contextmenu="showContextMenu($event)"
        />
      </div>

      <div
        v-if="contextMenu.visible"
        class="fixed z-50 min-w-52 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        @click.stop
      >
        <TrafficContextMenuSections
          :sections="contextMenuSections"
          label-prefix="trafficAnalysis.comparer.actions"
        />
      </div>
    </div>

    <div v-else class="flex flex-1 items-center justify-center text-sm text-base-content/60">
      {{ $t('trafficAnalysis.comparer.empty.noItems') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import CodeDiffViewer from '@/components/traffic/CodeDiffViewer.vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import { createRawRequestFromSource } from '@/components/traffic/intruder/http'
import TrafficContextMenuSections from './TrafficContextMenuSections.vue'
import { buildComparerActionMenuItems } from './trafficComparerActionMenuSupport'
import { buildTrafficContextMenuSections } from './trafficContextMenuSectionSupport'
import { useTrafficSendTargets } from './trafficSendTargets'
import {
  queueRepeaterTransfer,
  type TrafficComparerDraftRequestInput,
  type TrafficComparePayload,
} from './transfers'
import type { TrafficMessageViewTab } from './trafficDisplaySettings'
import {
  buildComparerDiffSummary,
  formatComparerText,
  inferComparerSideMeta,
  resolveComparerMeta,
  resolveComparerSideMeta,
} from './trafficComparerFormattingSupport'
import {
  buildComparerDraftPayload,
  canBuildComparerDraftPayload,
  createEmptyComparerDraft,
  type ComparerDraft,
} from './trafficComparerDraftSupport'

interface CompareItem extends TrafficComparePayload {
  id: string
}

interface PinnedBaseline {
  label: string
  text: string
  meta: NonNullable<TrafficComparePayload['leftMeta']>
}

const emit = defineEmits<{
  (e: 'sendToRepeater', request: { method: string; url: string; headers: Record<string, string>; body?: string }): void
}>()

const { t } = useI18n()
const { enabledTargets } = useTrafficSendTargets()
const items = ref<CompareItem[]>([])
const activeItemId = ref<string | null>(null)
const viewMode = ref<TrafficMessageViewTab>('pretty')
const pinnedBaseline = ref<PinnedBaseline | null>(null)
const showDraftComposer = ref(true)
const draftSequence = ref(1)
const draft = ref<ComparerDraft>(createDraftState())
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  mode: 'compare' as 'compare' | 'draft-left' | 'draft-right',
})

const currentItem = computed(() => items.value.find((item) => item.id === activeItemId.value) ?? null)
const compareMeta = computed(() => resolveComparerMeta(currentItem.value))
const leftMeta = computed(() => resolveComparerSideMeta(currentItem.value, 'left'))
const rightMeta = computed(() => resolveComparerSideMeta(currentItem.value, 'right'))
const diffMessageType = computed(() => {
  if (leftMeta.value.messageType === rightMeta.value.messageType) {
    return leftMeta.value.messageType
  }
  return 'generic'
})
const compareSourceLabel = computed(() =>
  t(`trafficAnalysis.comparer.badges.source.${compareMeta.value.source}`),
)
const compareKindLabel = computed(() =>
  t(`trafficAnalysis.comparer.badges.kind.${compareMeta.value.kind}`),
)
const compareMessageTypeLabel = computed(() =>
  t(`trafficAnalysis.comparer.badges.messageType.${diffMessageType.value}`),
)
const pinnedBaselineLabel = computed(() => pinnedBaseline.value?.label ?? '')
const displayedLeftText = computed(() => formatComparerText(currentItem.value?.leftText ?? '', viewMode.value))
const displayedRightText = computed(() => formatComparerText(currentItem.value?.rightText ?? '', viewMode.value))
const canSendLeftToRepeater = computed(() => leftMeta.value.messageType === 'request' && !!leftMeta.value.repeaterRequest)
const canSendRightToRepeater = computed(() => rightMeta.value.messageType === 'request' && !!rightMeta.value.repeaterRequest)
const canCreateDraftComparison = computed(() => canBuildComparerDraftPayload(draft.value))
const draftLeftMessageType = computed(() => inferComparerSideMeta(draft.value.leftText).messageType)
const draftRightMessageType = computed(() => inferComparerSideMeta(draft.value.rightText).messageType)
const comparerActionMenuItems = computed(() =>
  buildComparerActionMenuItems({
    actions: {
      copyLeft: () => copySide('left'),
      copyRight: () => copySide('right'),
      swapSides: swapCurrentItem,
      pinLeftBaseline: () => pinSideAsBaseline('left'),
      pinRightBaseline: () => pinSideAsBaseline('right'),
      clearPinnedBaseline,
      sendLeftToRepeater: () => sendSideToRepeater('left'),
      sendRightToRepeater: () => sendSideToRepeater('right'),
    },
    visible: {
      clearPinnedBaseline: !!pinnedBaseline.value,
      sendLeftToRepeater: enabledTargets.value.repeater,
      sendRightToRepeater: enabledTargets.value.repeater,
    },
    enabled: {
      sendLeftToRepeater: canSendLeftToRepeater.value,
      sendRightToRepeater: canSendRightToRepeater.value,
    },
  }),
)
const comparerToolbarActionMenuItems = computed(() => comparerActionMenuItems.value)
const compareContextMenuSections = computed(() =>
  buildTrafficContextMenuSections([
    {
      key: 'comparer-actions',
      items: comparerActionMenuItems.value.map((item) => ({
        ...item,
        onClick: () => handleContextMenuAction(item.onClick),
      })),
    },
  ]),
)
const draftContextMenuSections = computed(() => {
  const side = contextMenu.value.mode === 'draft-right' ? 'right' : 'left'
  const copyKey = side === 'left' ? 'copyLeft' : 'copyRight'
  const pasteKey = side === 'left' ? 'pasteLeft' : 'pasteRight'
  const draftText = side === 'left' ? draft.value.leftText : draft.value.rightText

  return buildTrafficContextMenuSections([
    {
      key: 'draft-actions',
      items: [
        {
          key: copyKey,
          iconClass: 'fas fa-copy text-secondary',
          labelKey: copyKey,
          onClick: () => handleContextMenuAction(() => copyDraftSide(side)),
          disabled: !draftText,
        },
        {
          key: pasteKey,
          iconClass: 'fas fa-paste text-accent',
          labelKey: pasteKey,
          onClick: () => handleContextMenuAction(() => pasteDraftSide(side)),
        },
        {
          key: 'clearDraft',
          iconClass: 'fas fa-eraser text-warning',
          labelKey: 'clearDraft',
          onClick: () => handleContextMenuAction(clearDraft),
        },
        {
          key: 'createComparison',
          iconClass: 'fas fa-not-equal text-primary',
          labelKey: 'createComparison',
          onClick: () => handleContextMenuAction(createComparisonFromDraft),
          disabled: !canCreateDraftComparison.value,
        },
      ],
    },
  ])
})
const contextMenuSections = computed(() =>
  contextMenu.value.mode === 'compare' ? compareContextMenuSections.value : draftContextMenuSections.value,
)
const diffSummary = computed(() => {
  if (!currentItem.value) {
    return { changedLines: 0, similarity: 100 }
  }
  return buildComparerDiffSummary(displayedLeftText.value, displayedRightText.value)
})

function createDraftState() {
  const index = draftSequence.value
  return createEmptyComparerDraft({
    defaultName: t('trafficAnalysis.comparer.draft.defaultName', { index }),
    leftLabel: t('trafficAnalysis.comparer.draft.leftLabel'),
    rightLabel: t('trafficAnalysis.comparer.draft.rightLabel'),
  })
}

function addComparison(payload: TrafficComparePayload) {
  const itemPayload = buildComparisonPayloadWithPinnedBaseline(payload)
  const item: CompareItem = {
    id: `compare-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    ...itemPayload,
  }
  items.value.push(item)
  activeItemId.value = item.id
  showDraftComposer.value = false
}

function closeItem(itemId: string) {
  items.value = items.value.filter((item) => item.id !== itemId)
  if (activeItemId.value === itemId) {
    activeItemId.value = items.value[0]?.id ?? null
  }
  if (!items.value.length) {
    showDraftComposer.value = true
  }
}

async function copySide(side: 'left' | 'right') {
  const item = currentItem.value
  if (!item) return

  const text = side === 'left' ? displayedLeftText.value : displayedRightText.value

  try {
    await navigator.clipboard.writeText(text)
    dialog.toast.success(
      t('trafficAnalysis.comparer.messages.copied', {
        label: side === 'left' ? item.leftLabel : item.rightLabel,
      }),
    )
  } catch {
    dialog.toast.error(t('trafficAnalysis.comparer.messages.copyFailed'))
  }
}

async function copyDraftSide(side: 'left' | 'right') {
  const text = side === 'left' ? draft.value.leftText : draft.value.rightText
  const label = side === 'left' ? draft.value.leftLabel : draft.value.rightLabel
  if (!text) return

  try {
    await navigator.clipboard.writeText(text)
    dialog.toast.success(
      t('trafficAnalysis.comparer.messages.copied', {
        label,
      }),
    )
  } catch {
    dialog.toast.error(t('trafficAnalysis.comparer.messages.copyFailed'))
  }
}

function openDraftComposer() {
  showDraftComposer.value = true
}

function hideContextMenu() {
  contextMenu.value.visible = false
  document.removeEventListener('click', hideContextMenu)
  document.removeEventListener('contextmenu', hideContextMenu)
}

function showContextMenu(event: MouseEvent) {
  if (!currentItem.value) return
  event.preventDefault()
  event.stopPropagation()
  contextMenu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - 240),
    y: Math.min(event.clientY, window.innerHeight - 260),
    mode: 'compare',
  }
  setTimeout(() => {
    document.addEventListener('click', hideContextMenu)
    document.addEventListener('contextmenu', hideContextMenu)
  }, 0)
}

function showDraftContextMenu(event: MouseEvent, side: 'left' | 'right') {
  event.preventDefault()
  event.stopPropagation()
  contextMenu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - 240),
    y: Math.min(event.clientY, window.innerHeight - 260),
    mode: side === 'left' ? 'draft-left' : 'draft-right',
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

function clearDraft() {
  draft.value = createDraftState()
}

async function pasteDraftSide(side: 'left' | 'right') {
  try {
    const text = await navigator.clipboard.readText()
    if (side === 'left') {
      draft.value.leftText = text
    } else {
      draft.value.rightText = text
    }
  } catch {
    dialog.toast.error(t('trafficAnalysis.comparer.messages.pasteFailed'))
  }
}

function createComparisonFromDraft() {
  if (!canCreateDraftComparison.value) {
    dialog.toast.warning(t('trafficAnalysis.comparer.messages.draftIncomplete'))
    return
  }

  const payload = buildComparerDraftPayload(
    draft.value,
    t('trafficAnalysis.comparer.draft.defaultName', { index: draftSequence.value }),
  )

  addComparison(payload)
  draftSequence.value += 1
  draft.value = createDraftState()
}

function setDraftSideText(side: 'left' | 'right', text: string, label?: string) {
  if (side === 'left') {
    draft.value.leftText = text
    if (label) {
      draft.value.leftLabel = label
    }
    return
  }

  draft.value.rightText = text
  if (label) {
    draft.value.rightLabel = label
  }
}

function addDraftRequest(input: TrafficComparerDraftRequestInput) {
  showDraftComposer.value = true

  const rawRequest = input.text ?? (input.request ? createRawRequestFromSource(input.request) : '')
  if (!rawRequest.trim()) {
    return
  }
  const requestedSide = input.side ?? 'auto'
  let targetSide: 'left' | 'right'

  if (requestedSide === 'left' || requestedSide === 'right') {
    targetSide = requestedSide
  } else if (!draft.value.leftText.trim()) {
    targetSide = 'left'
  } else if (!draft.value.rightText.trim()) {
    targetSide = 'right'
  } else {
    draft.value = createDraftState()
    targetSide = 'left'
  }

  if (input.name?.trim()) {
    draft.value.name = input.name.trim()
  }

  setDraftSideText(targetSide, rawRequest, input.label)
}

function swapCurrentItem() {
  const item = currentItem.value
  if (!item) return

  const nextLeftLabel = item.rightLabel
  const nextLeftText = item.rightText
  const nextLeftMeta = item.rightMeta
  item.rightLabel = item.leftLabel
  item.rightText = item.leftText
  item.rightMeta = item.leftMeta
  item.leftLabel = nextLeftLabel
  item.leftText = nextLeftText
  item.leftMeta = nextLeftMeta
}

function buildPinnedBaseline(side: 'left' | 'right'): PinnedBaseline | null {
  const item = currentItem.value
  if (!item) return null

  return {
    label: side === 'left' ? item.leftLabel : item.rightLabel,
    text: side === 'left' ? item.leftText : item.rightText,
    meta: side === 'left' ? leftMeta.value : rightMeta.value,
  }
}

function buildComparisonPayloadWithPinnedBaseline(payload: TrafficComparePayload): TrafficComparePayload {
  if (!pinnedBaseline.value) {
    return payload
  }

  return {
    ...payload,
    leftLabel: pinnedBaseline.value.label,
    leftText: pinnedBaseline.value.text,
    leftMeta: pinnedBaseline.value.meta,
    compareMeta: {
      source: payload.compareMeta?.source ?? 'generic',
      kind: 'baselineDiff',
    },
  }
}

function applyPinnedBaselineToCurrentItem() {
  const item = currentItem.value
  if (!item || !pinnedBaseline.value) return

  const previousLeftLabel = item.leftLabel
  const previousLeftText = item.leftText
  const previousLeftMeta = item.leftMeta
  const currentLeftMatches = item.leftLabel === pinnedBaseline.value.label
    && item.leftText === pinnedBaseline.value.text
  if (currentLeftMatches) return

  const currentRightMatches = item.rightLabel === pinnedBaseline.value.label
    && item.rightText === pinnedBaseline.value.text
  if (currentRightMatches) {
    item.rightLabel = previousLeftLabel
    item.rightText = previousLeftText
    item.rightMeta = previousLeftMeta
  }

  item.leftLabel = pinnedBaseline.value.label
  item.leftText = pinnedBaseline.value.text
  item.leftMeta = pinnedBaseline.value.meta
  item.compareMeta = {
    source: item.compareMeta?.source ?? 'generic',
    kind: 'baselineDiff',
  }
}

function pinSideAsBaseline(side: 'left' | 'right') {
  const baseline = buildPinnedBaseline(side)
  if (!baseline) return

  pinnedBaseline.value = baseline
  applyPinnedBaselineToCurrentItem()
  dialog.toast.success(
    t('trafficAnalysis.comparer.messages.pinnedBaseline', {
      label: baseline.label,
    }),
  )
}

function clearPinnedBaseline() {
  pinnedBaseline.value = null
  dialog.toast.success(t('trafficAnalysis.comparer.messages.clearedPinnedBaseline'))
}

function sendSideToRepeater(side: 'left' | 'right') {
  const item = currentItem.value
  if (!item) return

  const request = side === 'left' ? leftMeta.value.repeaterRequest : rightMeta.value.repeaterRequest
  if (!request) {
    dialog.toast.warning(t('trafficAnalysis.comparer.messages.notARequest'))
    return
  }

  emit('sendToRepeater', request)
  queueRepeaterTransfer(request)
  dialog.toast.success(
    t('trafficAnalysis.comparer.messages.sentToRepeater', {
      label: side === 'left' ? item.leftLabel : item.rightLabel,
    }),
  )
}

defineExpose({
  addComparison,
  addDraftRequest,
})
</script>
