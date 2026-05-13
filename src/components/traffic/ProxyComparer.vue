<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <div
      class="flex items-center gap-1.5 border-b border-base-300"
      :class="immersiveDrillModeEnabled ? IMMERSIVE_TRAFFIC_TOP_BAR_CLASS : 'bg-base-200 px-1.5 py-0.5'"
    >
      <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
        <div
          v-for="item in items"
          :key="item.id"
          class="flex items-center gap-1.5 rounded border border-base-300 px-2.5 py-1 text-sm"
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
      <button class="btn btn-xs btn-ghost" type="button" @click="openDraftComposer" :title="$t('trafficAnalysis.comparer.actions.newComparison')">
        <i class="fas fa-plus"></i>
        <span v-if="!immersiveDrillModeEnabled">{{ $t('trafficAnalysis.comparer.actions.newComparison') }}</span>
      </button>
    </div>

    <div v-if="showDraftComposer" class="flex min-h-0 flex-1 flex-col">
      <div
        class="border-b border-base-300"
        :class="immersiveDrillModeEnabled ? 'bg-base-200/75 px-2 py-1 backdrop-blur-sm' : 'bg-base-200 px-2.5 py-1.5'"
      >
        <div class="flex flex-wrap items-center gap-2">
          <span class="font-semibold text-sm">{{ $t('trafficAnalysis.comparer.draft.title') }}</span>
          <button class="btn btn-ghost btn-xs" type="button" @click="pasteDraftSide('left')">
            <i class="fas fa-paste"></i>
            <span v-if="!immersiveDrillModeEnabled">{{ $t('trafficAnalysis.comparer.actions.pasteLeft') }}</span>
          </button>
          <button class="btn btn-ghost btn-xs" type="button" @click="pasteDraftSide('right')">
            <i class="fas fa-paste"></i>
            <span v-if="!immersiveDrillModeEnabled">{{ $t('trafficAnalysis.comparer.actions.pasteRight') }}</span>
          </button>
          <button v-if="!immersiveDrillModeEnabled" class="btn btn-ghost btn-xs" type="button" @click="clearDraft">
            <i class="fas fa-eraser"></i>
            {{ $t('trafficAnalysis.comparer.actions.clearDraft') }}
          </button>
          <button class="btn btn-primary btn-xs" type="button" :disabled="!canCreateDraftComparison" @click="createComparisonFromDraft">
            <i class="fas fa-not-equal"></i>
            <span v-if="!immersiveDrillModeEnabled">{{ $t('trafficAnalysis.comparer.actions.createComparison') }}</span>
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
          <div class="border-b border-base-300 bg-base-200 px-2.5 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">
            {{ draft.leftLabel }}
          </div>
          <div class="min-h-0 flex-1" @contextmenu.capture.prevent="showDraftContextMenu($event, 'left')">
            <HttpMessageSurface
              ref="leftDraftEditor"
              v-model="draft.leftText"
              custom-context-menu
              show-search-bar
              :message-type="draftLeftMessageType"
              display-mode="raw"
              height="100%"
              :placeholder="$t('trafficAnalysis.comparer.draft.leftPlaceholder')"
              :state-key="buildComparerDraftStateKey('left')"
              :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
              :search-next-title="$t('trafficAnalysis.messageSearch.next')"
              :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
              :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
              :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
              :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
              :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
              :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
              @contextmenu="showDraftContextMenu($event, 'left')"
            />
          </div>
        </div>

        <div class="flex min-h-0 flex-col bg-base-100">
          <div class="border-b border-base-300 bg-base-200 px-2.5 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">
            {{ draft.rightLabel }}
          </div>
          <div class="min-h-0 flex-1" @contextmenu.capture.prevent="showDraftContextMenu($event, 'right')">
            <HttpMessageSurface
              ref="rightDraftEditor"
              v-model="draft.rightText"
              custom-context-menu
              show-search-bar
              :message-type="draftRightMessageType"
              display-mode="raw"
              height="100%"
              :placeholder="$t('trafficAnalysis.comparer.draft.rightPlaceholder')"
              :state-key="buildComparerDraftStateKey('right')"
              :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
              :search-next-title="$t('trafficAnalysis.messageSearch.next')"
              :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
              :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
              :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
              :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
              :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
              :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
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
      <div
        ref="compareToolbarRef"
        class="flex flex-wrap items-center gap-2 border-b border-base-300"
        :class="immersiveDrillModeEnabled ? 'bg-base-200/75 px-2 py-1 backdrop-blur-sm' : 'bg-base-200 px-2.5 py-1.5'"
      >
        <span :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ compareSourceLabel }}</span>
        <span :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ compareKindLabel }}</span>
        <span :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ compareMessageTypeLabel }}</span>
        <span v-if="pinnedBaseline" :class="[IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS, 'badge-accent']">
          {{ $t('trafficAnalysis.comparer.badges.pinnedBaseline', { label: pinnedBaselineLabel }) }}
        </span>
        <div class="tabs tabs-boxed tabs-xs bg-base-300 overflow-x-auto">
          <button
            type="button"
            :class="['tab tab-xs', viewMode === 'pretty' ? 'tab-active' : '']"
            @click="viewMode = 'pretty'"
          >
            {{ isCompareToolbarCompact ? comparePrettyTabShortLabel : $t('trafficAnalysis.comparer.tabs.pretty') }}
          </button>
          <button
            type="button"
            :class="['tab tab-xs', viewMode === 'raw' ? 'tab-active' : '']"
            @click="viewMode = 'raw'"
          >
            {{ isCompareToolbarCompact ? compareRawTabShortLabel : $t('trafficAnalysis.comparer.tabs.raw') }}
          </button>
        </div>
        <div class="tabs tabs-boxed tabs-xs bg-base-300 overflow-x-auto">
          <button
            type="button"
            :class="['tab tab-xs', compareRenderMode === 'diff' ? 'tab-active' : '']"
            @click="compareRenderMode = 'diff'"
          >
            {{ isCompareToolbarCompact ? compareDiffModeShortLabel : $t('trafficAnalysis.comparer.viewModes.diff') }}
          </button>
          <button
            type="button"
            :class="['tab tab-xs', compareRenderMode === 'plain' ? 'tab-active' : '']"
            @click="compareRenderMode = 'plain'"
          >
            {{ isCompareToolbarCompact ? comparePlainModeShortLabel : $t('trafficAnalysis.comparer.viewModes.plain') }}
          </button>
        </div>
        <TrafficMessageDisplayControls :compact="isCompareToolbarCompact" :show-line-endings="false" />
        <button
          v-if="!immersiveDrillModeEnabled"
          v-for="item in comparerToolbarActionMenuItems"
          :key="`comparer-toolbar-action-${item.key}`"
          class="btn btn-ghost btn-xs"
          type="button"
          :disabled="item.disabled"
          :title="$t(`trafficAnalysis.comparer.actions.${item.labelKey}`)"
          @click="item.onClick"
        >
          <i :class="item.iconClass.replace(' text-secondary', '').replace(' text-info', '').replace(' text-warning', '').replace(' text-primary', '')"></i>
          <span v-if="!isCompareToolbarCompact">{{ $t(`trafficAnalysis.comparer.actions.${item.labelKey}`) }}</span>
        </button>
        <div class="flex-1"></div>
        <span :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ currentItem.leftLabel }}: {{ displayedLeftText.length }}</span>
        <span :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ currentItem.rightLabel }}: {{ displayedRightText.length }}</span>
        <span v-if="!isCompareToolbarCompact" :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ $t('trafficAnalysis.comparer.labels.changedLines') }}: {{ diffSummary.changedLines }}</span>
        <span v-if="!isCompareToolbarCompact" :class="IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS">{{ $t('trafficAnalysis.comparer.labels.similarity') }}: {{ diffSummary.similarity }}%</span>
      </div>

      <div class="min-h-0 flex-1" :class="immersiveDrillModeEnabled ? 'p-1.5' : 'p-2.5'" @contextmenu.capture.prevent="showContextMenu($event)">
        <CodeDiffViewer
          v-if="compareRenderMode === 'diff'"
          :left-text="displayedLeftText"
          :right-text="displayedRightText"
          :message-type="diffMessageType"
          :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
          :search-next-title="$t('trafficAnalysis.messageSearch.next')"
          :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
          :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
          :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
          :search-active-side-title="$t('trafficAnalysis.messageSearch.activeSide')"
          :search-left-label="$t('trafficAnalysis.messageSearch.left')"
          :search-right-label="$t('trafficAnalysis.messageSearch.right')"
          :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
          :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
          :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
          @contextmenu="showContextMenu($event)"
        />
        <div v-else class="grid h-full min-h-0 gap-px rounded-[10px] border border-base-300 bg-base-300 md:grid-cols-2">
          <div class="flex min-h-0 flex-col bg-base-100" @contextmenu.capture.prevent="showContextMenu($event)">
            <div class="border-b border-base-300 bg-base-200 px-2.5 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">
              {{ currentItem.leftLabel }}
            </div>
            <div class="min-h-0 flex-1">
              <HttpMessageSurface
                :model-value="displayedLeftText"
                readonly
                custom-context-menu
                show-search-bar
                :message-type="leftMeta.messageType"
                :display-mode="viewMode"
                height="100%"
                :state-key="buildComparerPlainStateKey('left', viewMode)"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
                @contextmenu="showContextMenu($event)"
              />
            </div>
          </div>
          <div class="flex min-h-0 flex-col bg-base-100" @contextmenu.capture.prevent="showContextMenu($event)">
            <div class="border-b border-base-300 bg-base-200 px-2.5 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">
              {{ currentItem.rightLabel }}
            </div>
            <div class="min-h-0 flex-1">
              <HttpMessageSurface
                :model-value="displayedRightText"
                readonly
                custom-context-menu
                show-search-bar
                :message-type="rightMeta.messageType"
                :display-mode="viewMode"
                height="100%"
                :state-key="buildComparerPlainStateKey('right', viewMode)"
                :search-placeholder="$t('trafficAnalysis.messageSearch.placeholder')"
                :search-next-title="$t('trafficAnalysis.messageSearch.next')"
                :search-previous-title="$t('trafficAnalysis.messageSearch.previous')"
                :search-case-sensitive-title="$t('trafficAnalysis.messageSearch.caseSensitive')"
                :search-regexp-title="$t('trafficAnalysis.messageSearch.regexp')"
                :search-clear-title="$t('trafficAnalysis.messageSearch.clear')"
                :search-no-matches-text="$t('trafficAnalysis.messageSearch.noMatches')"
                :search-invalid-regexp-text="$t('trafficAnalysis.messageSearch.invalidRegexp')"
                :show-display-toolbar="false"
                @contextmenu="showContextMenu($event)"
              />
            </div>
          </div>
        </div>
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
        <div v-if="comparerTextCodecSubmenu" class="divider my-1 h-0"></div>
        <TrafficContextSubmenu
          v-if="comparerTextCodecSubmenu"
          :submenu="comparerTextCodecSubmenu"
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
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import CodeDiffViewer from '@/components/traffic/CodeDiffViewer.vue'
import {
  loadComparerStore,
  saveComparerStore,
  type PersistedTrafficComparerStore,
} from '@/api/trafficWorkbench'
import { buildComparerDraftStateKey, buildComparerPlainStateKey } from './trafficMessagePresentationSupport'
import TrafficMessageDisplayControls from '@/components/traffic/TrafficMessageDisplayControls.vue'
import { useI18n } from 'vue-i18n'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { dialog } from '@/composables/useDialog'
import {
  IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS,
  IMMERSIVE_TRAFFIC_TOP_BAR_CLASS,
} from './immersiveTrafficUi'
import { createRawRequestFromSource } from '@/components/traffic/intruder/http'
import TrafficContextMenuSections from './TrafficContextMenuSections.vue'
import TrafficContextSubmenu from './TrafficContextSubmenu.vue'
import { buildComparerActionMenuItems } from './trafficComparerActionMenuSupport'
import { buildTrafficContextMenuSections } from './trafficContextMenuSectionSupport'
import { buildTrafficTextCodecSubmenu, getTrafficTextCodecErrorMessage, hasNonEmptyTextSelection, replaceTrafficTextSelection, transformTrafficTextCodec, type TrafficTextCodecAction } from './trafficTextCodecSupport'
import { useTrafficSendTargets } from './trafficSendTargets'
import {
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
import { formatRepeaterPrettyRequest } from './trafficRepeaterPrettyRequestSupport'
import {
  buildComparerDraftPayload,
  canBuildComparerDraftPayload,
  createEmptyComparerDraft,
  type ComparerDraft,
} from './trafficComparerDraftSupport'
import type { HttpExchangeRequest } from './http/model'
import { useTrafficPaneCompactMode } from './useTrafficPaneCompactMode'

interface CompareItem extends TrafficComparePayload {
  id: string
}

interface PinnedBaseline {
  label: string
  text: string
  meta: NonNullable<TrafficComparePayload['leftMeta']>
}

const COMPARER_PERSIST_DEBOUNCE_MS = 250
const MAX_PERSISTED_COMPARE_ITEMS = 30

const emit = defineEmits<{
  (e: 'createDraft', request: HttpExchangeRequest): void
}>()

const { t, locale } = useI18n()
const { enabledTargets } = useTrafficSendTargets()
const {
  panelRef: compareToolbarRef,
  isCompact: isCompareToolbarCompact,
} = useTrafficPaneCompactMode(920)
const items = ref<CompareItem[]>([])
const activeItemId = ref<string | null>(null)
const viewMode = ref<TrafficMessageViewTab>('pretty')
const compareRenderMode = ref<'diff' | 'plain'>('diff')
const pinnedBaseline = ref<PinnedBaseline | null>(null)
const showDraftComposer = ref(true)
const draftSequence = ref(1)
const draft = ref<ComparerDraft>(createDraftState())
const leftDraftEditor = ref<InstanceType<typeof HttpMessageSurface> | null>(null)
const rightDraftEditor = ref<InstanceType<typeof HttpMessageSurface> | null>(null)
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  mode: 'compare' as 'compare' | 'draft-left' | 'draft-right',
  selection: null as { from: number; to: number } | null,
})
const pendingComparisonInputs: TrafficComparePayload[] = []
const pendingDraftRequestInputs: TrafficComparerDraftRequestInput[] = []
let comparerHydrated = false
let comparerHydrationPromise: Promise<void> | null = null
let comparerPersistTimer: ReturnType<typeof setTimeout> | null = null

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
const comparePrettyTabShortLabel = computed(() => locale.value.startsWith('zh') ? '格式' : 'Fmt')
const compareRawTabShortLabel = computed(() => locale.value.startsWith('zh') ? '原始' : 'Raw')
const compareDiffModeShortLabel = computed(() => locale.value.startsWith('zh') ? '差异' : 'Diff')
const comparePlainModeShortLabel = computed(() => 'Plain')
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
      sendLeftToRepeater: enabledTargets.value.draft,
      sendRightToRepeater: enabledTargets.value.draft,
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
const comparerTextCodecSubmenu = computed(() => (
  contextMenu.value.mode === 'compare'
    ? null
    : buildTrafficTextCodecSubmenu({
        disabled: !hasNonEmptyTextSelection(contextMenu.value.selection),
        onClick: action => handleContextMenuAction(() => applyTextCodecToDraftSide(action)),
      })
))
const diffSummary = computed(() => {
  if (!currentItem.value) {
    return { changedLines: 0, similarity: 100 }
  }
  return buildComparerDiffSummary(displayedLeftText.value, displayedRightText.value)
})

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value))
}

function normalizeViewMode(value: unknown): TrafficMessageViewTab {
  return value === 'raw' ? 'raw' : 'pretty'
}

function normalizeCompareRenderMode(value: unknown): 'diff' | 'plain' {
  return value === 'plain' ? 'plain' : 'diff'
}

function normalizeCompareItem(value: unknown): CompareItem | null {
  if (!isRecord(value)) {
    return null
  }

  const id = typeof value.id === 'string' ? value.id : ''
  const name = typeof value.name === 'string' ? value.name : ''
  const leftLabel = typeof value.leftLabel === 'string' ? value.leftLabel : ''
  const rightLabel = typeof value.rightLabel === 'string' ? value.rightLabel : ''
  const leftText = typeof value.leftText === 'string' ? value.leftText : ''
  const rightText = typeof value.rightText === 'string' ? value.rightText : ''
  if (!id || !leftLabel || !rightLabel) {
    return null
  }

  return {
    ...(value as unknown as TrafficComparePayload),
    id,
    name,
    leftLabel,
    rightLabel,
    leftText,
    rightText,
  }
}

function normalizePinnedBaseline(value: unknown): PinnedBaseline | null {
  if (!isRecord(value)) {
    return null
  }

  if (
    typeof value.label !== 'string'
    || typeof value.text !== 'string'
    || !isRecord(value.meta)
  ) {
    return null
  }

  return {
    label: value.label,
    text: value.text,
    meta: value.meta as unknown as NonNullable<TrafficComparePayload['leftMeta']>,
  }
}

function normalizeDraft(value: unknown): ComparerDraft {
  if (!isRecord(value)) {
    return createDraftState()
  }

  return {
    name: typeof value.name === 'string' ? value.name : createDraftState().name,
    leftLabel: typeof value.leftLabel === 'string' ? value.leftLabel : t('trafficAnalysis.comparer.draft.leftLabel'),
    rightLabel: typeof value.rightLabel === 'string' ? value.rightLabel : t('trafficAnalysis.comparer.draft.rightLabel'),
    leftText: typeof value.leftText === 'string' ? value.leftText : '',
    rightText: typeof value.rightText === 'string' ? value.rightText : '',
  }
}

function buildPersistedComparerStore(): PersistedTrafficComparerStore {
  return {
    activeItemId: activeItemId.value,
    items: items.value.slice(-MAX_PERSISTED_COMPARE_ITEMS).map(item => ({ ...item })),
    viewMode: viewMode.value,
    compareRenderMode: compareRenderMode.value,
    pinnedBaseline: pinnedBaseline.value ? { ...pinnedBaseline.value } : null,
    showDraftComposer: showDraftComposer.value,
    draftSequence: draftSequence.value,
    draft: { ...draft.value },
  }
}

function scheduleComparerPersist() {
  if (!comparerHydrated) {
    return
  }

  if (comparerPersistTimer) {
    clearTimeout(comparerPersistTimer)
  }

  comparerPersistTimer = setTimeout(() => {
    comparerPersistTimer = null
    void saveComparerStore(buildPersistedComparerStore()).catch(error => {
      console.error('[ProxyComparer] Failed to persist comparer store:', error)
    })
  }, COMPARER_PERSIST_DEBOUNCE_MS)
}

function flushPendingComparerInputs() {
  const comparisons = pendingComparisonInputs.splice(0)
  const drafts = pendingDraftRequestInputs.splice(0)
  comparisons.forEach(applyComparison)
  drafts.forEach(applyDraftRequest)
}

async function ensureComparerHydrated() {
  if (comparerHydrated) {
    return
  }

  if (!comparerHydrationPromise) {
    comparerHydrationPromise = loadPersistedComparerStore()
  }
  await comparerHydrationPromise
}

async function loadPersistedComparerStore() {
  try {
    const store = await loadComparerStore()
    const restoredItems = Array.isArray(store.items)
      ? store.items.map(normalizeCompareItem).filter((item): item is CompareItem => Boolean(item))
      : []
    items.value = restoredItems
    activeItemId.value = restoredItems.some(item => item.id === store.activeItemId)
      ? store.activeItemId
      : restoredItems[0]?.id ?? null
    viewMode.value = normalizeViewMode(store.viewMode)
    compareRenderMode.value = normalizeCompareRenderMode(store.compareRenderMode)
    pinnedBaseline.value = normalizePinnedBaseline(store.pinnedBaseline)
    showDraftComposer.value = typeof store.showDraftComposer === 'boolean'
      ? store.showDraftComposer
      : restoredItems.length === 0
    draftSequence.value = typeof store.draftSequence === 'number' && Number.isFinite(store.draftSequence)
      ? Math.max(1, Math.round(store.draftSequence))
      : 1
    draft.value = normalizeDraft(store.draft)
  } catch (error) {
    console.error('[ProxyComparer] Failed to hydrate comparer store:', error)
  } finally {
    comparerHydrated = true
    flushPendingComparerInputs()
  }
}

function createDraftState() {
  const index = draftSequence.value
  return createEmptyComparerDraft({
    defaultName: t('trafficAnalysis.comparer.draft.defaultName', { index }),
    leftLabel: t('trafficAnalysis.comparer.draft.leftLabel'),
    rightLabel: t('trafficAnalysis.comparer.draft.rightLabel'),
  })
}

function createCompareItem(payload: TrafficComparePayload): CompareItem {
  const itemPayload = buildComparisonPayloadWithPinnedBaseline(payload)
  return {
    id: `compare-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    ...itemPayload,
  }
}

function applyComparison(payload: TrafficComparePayload) {
  const item = createCompareItem(payload)
  items.value.push(item)
  activeItemId.value = item.id
  showDraftComposer.value = false
}

function addComparison(payload: TrafficComparePayload) {
  if (!comparerHydrated) {
    pendingComparisonInputs.push(payload)
    void ensureComparerHydrated()
    return
  }

  applyComparison(payload)
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
    selection: null,
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
    selection: (side === 'left' ? leftDraftEditor.value : rightDraftEditor.value)?.getSelectionRange?.() ?? null,
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

async function applyTextCodecToDraftSide(action: TrafficTextCodecAction) {
  const selection = contextMenu.value.selection
  if (!hasNonEmptyTextSelection(selection)) {
    dialog.toast.warning(t('trafficAnalysis.textCodec.noSelection'))
    return
  }

  const side = contextMenu.value.mode === 'draft-right' ? 'right' : 'left'
  const currentText = side === 'left' ? draft.value.leftText : draft.value.rightText
  try {
    const replacement = transformTrafficTextCodec(currentText.slice(selection.from, selection.to), action)
    const next = replaceTrafficTextSelection(currentText, selection, replacement)
    if (side === 'left') {
      draft.value.leftText = next.content
    } else {
      draft.value.rightText = next.content
    }

    await nextTick()
    const editor = side === 'left' ? leftDraftEditor.value : rightDraftEditor.value
    editor?.setSelection?.(next.selectionStart, next.selectionEnd)
    editor?.focus?.()
    dialog.toast.success(t('trafficAnalysis.textCodec.applied', {
      action: t(`trafficAnalysis.comparer.actions.${action.labelKey}`),
    }))
  } catch (error) {
    dialog.toast.error(t('trafficAnalysis.textCodec.failed', {
      error: getTrafficTextCodecErrorMessage(error),
    }))
  }
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

function applyDraftRequest(input: TrafficComparerDraftRequestInput) {
  showDraftComposer.value = true

  const baseRequestText = input.text ?? (input.request ? createRawRequestFromSource(input.request) : '')
  const requestText = input.request?.preferredRequestView === 'pretty'
    ? formatRepeaterPrettyRequest(baseRequestText)
    : baseRequestText

  if (!requestText.trim()) {
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

  setDraftSideText(targetSide, requestText, input.label)
}

function addDraftRequest(input: TrafficComparerDraftRequestInput) {
  if (!comparerHydrated) {
    pendingDraftRequestInputs.push(input)
    void ensureComparerHydrated()
    return
  }

  applyDraftRequest(input)
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

  emit('createDraft', request)
  dialog.toast.success(
    t('trafficAnalysis.comparer.messages.draftCreated', {
      label: side === 'left' ? item.leftLabel : item.rightLabel,
    }),
  )
}

watch(
  [
    items,
    activeItemId,
    viewMode,
    compareRenderMode,
    pinnedBaseline,
    showDraftComposer,
    draftSequence,
    draft,
  ],
  scheduleComparerPersist,
  { deep: true },
)

onMounted(() => {
  void ensureComparerHydrated()
})

onUnmounted(() => {
  if (comparerPersistTimer) {
    clearTimeout(comparerPersistTimer)
    comparerPersistTimer = null
    if (comparerHydrated) {
      void saveComparerStore(buildPersistedComparerStore()).catch(error => {
        console.error('[ProxyComparer] Failed to persist comparer store on unmount:', error)
      })
    }
  }
  document.removeEventListener('click', hideContextMenu)
  document.removeEventListener('contextmenu', hideContextMenu)
})

defineExpose({
  addComparison,
  addDraftRequest,
})
</script>
