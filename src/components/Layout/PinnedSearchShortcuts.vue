<template>
  <div v-if="shortcuts.length > 0" class="space-y-2">
    <div class="flex items-center justify-between gap-3">
      <div class="text-sm font-medium text-base-content/75">{{ title }}</div>
      <div class="flex items-center gap-2">
        <span class="text-xs text-base-content/45">
          {{ isManageMode ? `已选 ${selectedShortcutCount} 项` : dragHintText }}
        </span>
        <button class="btn btn-ghost btn-xs" @click="toggleManageMode">
          {{ isManageMode ? '完成' : '管理' }}
        </button>
      </div>
    </div>
    <div
      v-if="isManageMode"
      class="space-y-3 rounded-2xl border border-base-300 bg-base-200/40 px-3 py-3"
    >
      <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div class="text-xs text-base-content/55">管理态下可批量选择，也可用上下按钮精确调整组内顺序</div>
        <label class="input input-sm input-bordered flex items-center gap-2 lg:w-72">
          <i class="fas fa-search text-base-content/40"></i>
          <input
            v-model="manageSearchQuery"
            type="text"
            class="grow"
            placeholder="搜索固定快捷入口"
          />
        </label>
      </div>
      <PinnedSearchShortcutTagPresetManager
        :tag-presets="tagPresets"
        :active-tags="manageTagFilter ? [manageTagFilter] : []"
        :removable-tags="customTagPresets"
        placeholder="新增自定义标签预设"
        @toggle="manageTagFilter = manageTagFilter === $event ? '' : $event"
        @add="addTagPreset"
        @remove="removeTagPresetWithMigration"
        @rename="renameTagPreset"
        @reorder="reorderTagPreset"
      />
      <div v-if="tagStats.length > 0" class="space-y-2">
        <div class="text-xs font-medium text-base-content/55">标签统计</div>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="stat in tagStats"
            :key="stat.tag"
            class="badge border transition-colors"
            :class="manageTagFilter === stat.tag
              ? 'border-primary bg-primary/10 text-primary'
              : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary/40 hover:text-primary'"
            @click="selectShortcutsByTag(stat.tag)"
          >
            #{{ stat.tag }} {{ stat.selectedCount > 0 ? `${stat.selectedCount}/${stat.totalCount}` : stat.totalCount }}
          </button>
        </div>
      </div>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <button
            v-for="option in manageFilterOptions"
            :key="option.value"
            class="badge border transition-colors"
            :class="manageGroupFilter === option.value
              ? 'border-primary bg-primary/10 text-primary'
              : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary/40 hover:text-primary'"
            @click="manageGroupFilter = option.value"
          >
            {{ option.label }}
          </button>
        </div>
        <div v-if="availableTags.length > 0" class="flex flex-wrap items-center gap-2">
          <button
            class="badge border transition-colors"
            :class="manageTagFilter === ''
              ? 'border-primary bg-primary/10 text-primary'
              : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary/40 hover:text-primary'"
            @click="manageTagFilter = ''"
          >
            全部标签
          </button>
          <button
            v-for="tag in availableTags"
            :key="tag"
            class="badge border transition-colors"
            :class="manageTagFilter === tag
              ? 'border-primary bg-primary/10 text-primary'
              : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary/40 hover:text-primary'"
            @click="manageTagFilter = tag"
          >
            #{{ tag }}
          </button>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button class="btn btn-ghost btn-xs" :disabled="visibleShortcutIds.length === 0" @click="selectAllShortcuts">
            全选当前结果
          </button>
          <button class="btn btn-ghost btn-xs" :disabled="selectedShortcutCount === 0" @click="clearSelectedShortcuts">
            清空选择
          </button>
          <button class="btn btn-error btn-xs" :disabled="selectedShortcutCount === 0" @click="removeSelectedShortcuts">
            取消固定所选
          </button>
        </div>
      </div>
      <div v-if="selectedShortcutCount > 0" class="space-y-2 rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="text-xs font-medium text-base-content/55">批量标签操作</div>
          <div class="flex items-center gap-2">
            <button
              class="btn btn-ghost btn-xs"
              :class="batchTagActionMode === 'add' ? 'text-primary' : ''"
              @click="batchTagActionMode = 'add'"
            >
              追加标签
            </button>
            <button
              class="btn btn-ghost btn-xs"
              :class="batchTagActionMode === 'remove' ? 'text-primary' : ''"
              @click="batchTagActionMode = 'remove'"
            >
              移除标签
            </button>
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="tag in tagPresets"
            :key="`batch-${tag}`"
            class="badge border transition-colors"
            :class="batchTagActionMode === 'add'
              ? 'border-base-300 bg-base-100 text-base-content/70 hover:border-primary hover:text-primary'
              : 'border-base-300 bg-base-100 text-base-content/70 hover:border-error hover:text-error'"
            @click="applyBatchTagAction(tag)"
          >
            {{ batchTagActionMode === 'add' ? '+' : '-' }} #{{ tag }}
          </button>
        </div>
      </div>
      <PinnedSearchShortcutBatchHistory
        :entries="batchHistoryEntries"
        @undo="undoBatchHistoryEntryAndRestore"
        @clear="clearBatchHistoryEntries"
      />
    </div>
    <div
      v-for="group in visibleGroupedShortcuts"
      :key="group.key"
      class="space-y-2"
    >
      <div class="flex items-center justify-between gap-3 px-1">
        <button
          class="flex items-center gap-2 text-left text-[11px] font-semibold uppercase tracking-wider text-base-content/45 transition-colors hover:text-primary"
          @click="handleToggleGroupCollapsed(group.key)"
        >
          <i
            class="fas fa-chevron-right text-[10px] transition-transform"
            :class="isGroupCollapsed(group.key) ? '' : 'rotate-90'"
          ></i>
          <span>{{ group.label }}</span>
          <span class="badge badge-ghost badge-sm">{{ group.items.length }}</span>
        </button>
        <span v-if="isGroupCollapsed(group.key)" class="text-[11px] text-base-content/35">
          已折叠
        </span>
      </div>
      <div v-if="!isGroupCollapsed(group.key) && isManageMode" class="space-y-2">
        <div
          v-for="(shortcut, index) in group.items"
          :key="shortcut.id"
          class="flex items-center gap-3 rounded-2xl border bg-base-100 px-3 py-3 transition-colors"
          :class="isShortcutSelected(shortcut.id) ? 'border-primary bg-primary/10' : 'border-base-300'"
        >
          <button
            class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border text-[11px] transition-colors"
            :class="isShortcutSelected(shortcut.id)
              ? 'border-primary bg-primary text-primary-content'
              : 'border-base-300 text-base-content/35'"
            @click.stop="toggleShortcutSelection(shortcut.id)"
          >
            <i :class="isShortcutSelected(shortcut.id) ? 'fas fa-check' : 'fas fa-circle'" />
          </button>

          <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-base-200 text-xs font-semibold text-base-content/55">
            {{ index + 1 }}
          </div>

          <button
            class="min-w-0 flex-1 text-left transition-colors hover:text-primary"
            @click="handleShortcutClick(shortcut)"
          >
            <div class="flex items-start gap-3">
              <div class="mt-0.5 text-base-content/65">
                <i :class="shortcut.icon"></i>
              </div>
              <div class="min-w-0">
                <div class="truncate text-sm font-medium">{{ shortcut.title }}</div>
                <div v-if="shortcut.description" class="mt-0.5 line-clamp-2 text-xs text-base-content/55">
                  {{ shortcut.description }}
                </div>
                <div v-if="shortcut.pinnedTags.length > 0" class="mt-2 flex flex-wrap gap-1">
                  <span
                    v-for="tag in shortcut.pinnedTags"
                    :key="`${shortcut.id}-${tag}`"
                    class="badge badge-outline badge-sm text-[11px]"
                  >
                    #{{ tag }}
                  </span>
                </div>
                <div class="mt-2 flex flex-wrap gap-1">
                  <button
                    v-for="tag in tagPresets"
                    :key="`${shortcut.id}-preset-${tag}`"
                    class="badge border transition-colors"
                    :class="shortcut.pinnedTags.includes(tag)
                      ? 'border-primary bg-primary/10 text-primary'
                      : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary hover:text-primary'"
                    @click.stop="toggleShortcutTag(shortcut, tag)"
                  >
                    #{{ tag }}
                  </button>
                </div>
              </div>
            </div>
          </button>

          <div class="flex items-center gap-1">
            <button
              class="btn btn-ghost btn-xs"
              :disabled="!canMoveShortcut(group, shortcut.id, -1)"
              @click.stop="moveShortcut(group, shortcut.id, -1)"
            >
              <i class="fas fa-arrow-up"></i>
            </button>
            <button
              class="btn btn-ghost btn-xs"
              :disabled="!canMoveShortcut(group, shortcut.id, 1)"
              @click.stop="moveShortcut(group, shortcut.id, 1)"
            >
              <i class="fas fa-arrow-down"></i>
            </button>
          </div>
        </div>
      </div>
      <div v-else-if="!isGroupCollapsed(group.key)" class="flex flex-wrap gap-2">
        <div
          v-for="shortcut in group.items"
          :key="shortcut.id"
          class="flex min-w-[14rem] max-w-[22rem] items-start gap-2 rounded-2xl border bg-base-100 px-3 py-2 transition-colors"
          :class="getShortcutCardClass(shortcut.id)"
          :draggable="!isManageMode"
          @dragstart="handleDragStart(shortcut.id)"
          @dragenter.prevent="handleDragEnter(shortcut.id)"
          @dragover.prevent="handleDragEnter(shortcut.id)"
          @drop.prevent="handleDrop(shortcut.id)"
          @dragend="resetDragState"
        >
          <button
            v-if="isManageMode"
            class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border text-[11px] transition-colors"
            :class="isShortcutSelected(shortcut.id)
              ? 'border-primary bg-primary text-primary-content'
              : 'border-base-300 text-base-content/35'"
            @click.stop="toggleShortcutSelection(shortcut.id)"
          >
            <i :class="isShortcutSelected(shortcut.id) ? 'fas fa-check' : 'fas fa-circle'" />
          </button>
          <div v-else class="mt-1 text-base-content/30">
            <i class="fas fa-grip-lines"></i>
          </div>
          <button
            class="min-w-0 flex-1 text-left transition-colors hover:text-primary"
            @click="handleShortcutClick(shortcut)"
          >
            <div class="flex items-start gap-2">
              <i :class="shortcut.icon"></i>
              <div class="min-w-0">
                <div class="truncate text-sm font-medium">{{ shortcut.title }}</div>
                <div v-if="shortcut.description" class="mt-0.5 line-clamp-2 text-xs text-base-content/55">
                  {{ shortcut.description }}
                </div>
                <div v-if="shortcut.pinnedTags.length > 0" class="mt-2 flex flex-wrap gap-1">
                  <span
                    v-for="tag in shortcut.pinnedTags"
                    :key="`${shortcut.id}-${tag}`"
                    class="badge badge-outline badge-sm text-[11px]"
                  >
                    #{{ tag }}
                  </span>
                </div>
              </div>
            </div>
          </button>
          <div v-if="!isManageMode" class="flex items-center gap-1">
            <SearchShortcutEditButton @edit="$emit('edit', shortcut)" />
            <SearchShortcutPinButton :pinned="true" @toggle="$emit('toggle', shortcut)" />
          </div>
        </div>
      </div>
    </div>
    <div
      v-if="isManageMode && visibleGroupedShortcuts.length === 0"
      class="rounded-2xl border border-dashed border-base-300 px-4 py-8 text-center text-sm text-base-content/55"
    >
      没有匹配的固定快捷入口，换个关键词或切换分组试试。
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { usePinnedSearchShortcutPreferences } from '@/composables/usePinnedSearchShortcutPreferences'
import { usePinnedSearchShortcutTagPresets } from '@/composables/usePinnedSearchShortcutTagPresets'
import {
  usePinnedSearchShortcutBatchHistory,
} from '@/services/pinnedSearchShortcutBatchHistory'
import {
  clonePinnedSearchShortcutSnapshots,
  getPinnedSearchShortcutGroupKey,
  groupPinnedSearchShortcuts,
  type PinnedSearchShortcutGroupKey,
  type PinnedSearchShortcutGroup,
  type PinnedSearchShortcutSnapshot,
  type ResolvedPinnedSearchShortcutEntry,
} from '@/services/pinnedSearchShortcuts'
import {
  canMovePinnedSearchShortcut,
  getPinnedSearchShortcutMoveTarget,
  type PinnedSearchShortcutMoveDirection,
} from '@/services/pinnedSearchShortcutOrdering'
import {
  filterPinnedSearchShortcutGroups,
  type PinnedSearchShortcutManageFilter,
} from '@/services/pinnedSearchShortcutFiltering'
import {
  buildPinnedSearchShortcutTagPresets,
  togglePinnedSearchShortcutTag,
} from '@/services/pinnedSearchShortcutTags'
import {
  buildPinnedSearchShortcutTagStats,
  collectPinnedSearchShortcutIdsByTag,
} from '@/services/pinnedSearchShortcutTagStats'
import PinnedSearchShortcutBatchHistory from './PinnedSearchShortcutBatchHistory.vue'
import PinnedSearchShortcutTagPresetManager from './PinnedSearchShortcutTagPresetManager.vue'
import SearchShortcutEditButton from './SearchShortcutEditButton.vue'
import SearchShortcutPinButton from './SearchShortcutPinButton.vue'

const draggedId = ref<string | null>(null)
const dragOverId = ref<string | null>(null)
const invalidDragOverId = ref<string | null>(null)
const isManageMode = ref(false)
const selectedShortcutIds = ref<string[]>([])
const manageSearchQuery = ref('')
const manageGroupFilter = ref<PinnedSearchShortcutManageFilter>('all')
const manageTagFilter = ref('')
const batchTagActionMode = ref<'add' | 'remove'>('add')
const {
  customTagPresets,
  addCustomTagPreset,
  removeCustomTagPreset,
  renameCustomTagPreset,
  reorderCustomTagPresets,
} = usePinnedSearchShortcutTagPresets()
const { entries: batchHistoryEntries, undoEntry: undoBatchHistoryEntry, clearEntries: clearBatchHistoryEntries } =
  usePinnedSearchShortcutBatchHistory()

const props = defineProps<{
  title: string
  shortcuts: ResolvedPinnedSearchShortcutEntry[]
}>()

const emit = defineEmits<{
  select: [entry: ResolvedPinnedSearchShortcutEntry]
  edit: [entry: ResolvedPinnedSearchShortcutEntry]
  toggle: [entry: ResolvedPinnedSearchShortcutEntry]
  reorder: [activeId: string, targetId: string]
  removeMany: [entryIds: string[]]
  updateTags: [payload: { id: string; customTags: string[] }]
  updateManyTags: [payload: { entryIds: string[]; tag: string; mode: 'add' | 'remove' }]
  replaceTag: [payload: { currentTag: string; nextTag: string }]
  removeTag: [payload: { tag: string; replacementTag?: string }]
  restoreHistory: [snapshots: PinnedSearchShortcutSnapshot[]]
}>()

const groupedShortcuts = computed(() => groupPinnedSearchShortcuts(props.shortcuts))
const manageScopedGroups = computed(() =>
  filterPinnedSearchShortcutGroups(groupedShortcuts.value, {
    query: manageSearchQuery.value,
    groupFilter: manageGroupFilter.value,
    tagFilter: '',
  }),
)
const visibleGroupedShortcuts = computed(() =>
  isManageMode.value
    ? filterPinnedSearchShortcutGroups(groupedShortcuts.value, {
        query: manageSearchQuery.value,
        groupFilter: manageGroupFilter.value,
        tagFilter: manageTagFilter.value,
      })
    : groupedShortcuts.value,
)
const { isGroupCollapsed, toggleGroupCollapsed } = usePinnedSearchShortcutPreferences()
const shortcutIds = computed(() => props.shortcuts.map(shortcut => shortcut.id))
const visibleShortcutIds = computed(() =>
  visibleGroupedShortcuts.value.flatMap(group => group.items.map(shortcut => shortcut.id)),
)
const manageScopedShortcuts = computed(() =>
  manageScopedGroups.value.flatMap(group => group.items),
)
const shortcutGroupKeyById = computed(() =>
  new Map(props.shortcuts.map(shortcut => [shortcut.id, getPinnedSearchShortcutGroupKey(shortcut)])),
)
const selectedShortcutCount = computed(() => selectedShortcutIds.value.length)
const tagPresets = computed(() => buildPinnedSearchShortcutTagPresets({
  customTags: customTagPresets.value,
  dynamicTags: props.shortcuts,
}))
const availableTags = computed(() =>
  tagStats.value.map(stat => stat.tag),
)
const tagStats = computed(() => buildPinnedSearchShortcutTagStats(manageScopedShortcuts.value, selectedShortcutIds.value))
const manageFilterOptions: Array<{ value: PinnedSearchShortcutManageFilter; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'command', label: '命令' },
  { value: 'resource', label: '页面与结果' },
]
const dragHintText = computed(() =>
  invalidDragOverId.value
    ? '命令与页面结果分组独立，仅支持组内拖拽排序'
    : '固定后会优先展示，仅支持组内拖拽排序',
)

watch(shortcutIds, (nextShortcutIds) => {
  selectedShortcutIds.value = selectedShortcutIds.value.filter(shortcutId => nextShortcutIds.includes(shortcutId))

  if (nextShortcutIds.length === 0) {
    isManageMode.value = false
  }
}, { immediate: true })

watch([manageSearchQuery, manageGroupFilter, manageTagFilter], () => {
  if (!isManageMode.value) {
    return
  }

  clearSelectedShortcuts()
})

const toggleManageMode = () => {
  isManageMode.value = !isManageMode.value

  if (!isManageMode.value) {
    clearSelectedShortcuts()
    manageSearchQuery.value = ''
    manageGroupFilter.value = 'all'
    manageTagFilter.value = ''
  }
}

const clearSelectedShortcuts = () => {
  selectedShortcutIds.value = []
}

const selectAllShortcuts = () => {
  selectedShortcutIds.value = [...visibleShortcutIds.value]
}

const selectShortcutsByTag = (tag: string) => {
  manageTagFilter.value = tag
  selectedShortcutIds.value = collectPinnedSearchShortcutIdsByTag(manageScopedShortcuts.value, tag)
}

const isShortcutSelected = (shortcutId: string) => selectedShortcutIds.value.includes(shortcutId)

const toggleShortcutSelection = (shortcutId: string) => {
  selectedShortcutIds.value = isShortcutSelected(shortcutId)
    ? selectedShortcutIds.value.filter(item => item !== shortcutId)
    : [...selectedShortcutIds.value, shortcutId]
}

const removeSelectedShortcuts = () => {
  if (selectedShortcutIds.value.length === 0) {
    return
  }

  emit('removeMany', selectedShortcutIds.value)
  clearSelectedShortcuts()
  isManageMode.value = false
}

const applyBatchTagAction = (tag: string) => {
  if (selectedShortcutIds.value.length === 0) {
    return
  }

  emit('updateManyTags', {
    entryIds: selectedShortcutIds.value,
    tag,
    mode: batchTagActionMode.value,
  })
}

const undoBatchHistoryEntryAndRestore = (entryId: string) => {
  const restoredSnapshots = undoBatchHistoryEntry(entryId)
  if (!restoredSnapshots) {
    return
  }

  emit('restoreHistory', clonePinnedSearchShortcutSnapshots(restoredSnapshots))
}

const toggleShortcutTag = (shortcut: ResolvedPinnedSearchShortcutEntry, tag: string) => {
  emit('updateTags', {
    id: shortcut.id,
    customTags: togglePinnedSearchShortcutTag(shortcut.pinnedTags, tag),
  })
}

const addTagPreset = (tag: string) => {
  addCustomTagPreset(tag)
  manageTagFilter.value = tag
}

const removeTagPreset = (tag: string) => {
  removeCustomTagPreset(tag)

  if (manageTagFilter.value === tag && !availableTags.value.includes(tag)) {
    manageTagFilter.value = ''
  }
}

const renameTagPreset = (payload: { currentTag: string; nextTag: string }) => {
  renameCustomTagPreset(payload.currentTag, payload.nextTag)
  emit('replaceTag', payload)

  if (manageTagFilter.value === payload.currentTag) {
    manageTagFilter.value = payload.nextTag
  }
}

const reorderTagPreset = (payload: { activeTag: string; targetTag: string }) => {
  reorderCustomTagPresets(payload.activeTag, payload.targetTag)
}

const removeTagPresetWithMigration = (payload: { tag: string; replacementTag?: string }) => {
  removeTagPreset(payload.tag)
  emit('removeTag', payload)
}

const handleShortcutClick = (shortcut: ResolvedPinnedSearchShortcutEntry) => {
  if (isManageMode.value) {
    toggleShortcutSelection(shortcut.id)
    return
  }

  emit('select', shortcut)
}

const canMoveShortcut = (
  group: PinnedSearchShortcutGroup<ResolvedPinnedSearchShortcutEntry>,
  shortcutId: string,
  direction: PinnedSearchShortcutMoveDirection,
) => canMovePinnedSearchShortcut(group, shortcutId, direction)

const moveShortcut = (
  group: PinnedSearchShortcutGroup<ResolvedPinnedSearchShortcutEntry>,
  shortcutId: string,
  direction: PinnedSearchShortcutMoveDirection,
) => {
  const targetId = getPinnedSearchShortcutMoveTarget(group, shortcutId, direction)
  if (!targetId) {
    return
  }

  emit('reorder', shortcutId, targetId)
}

const getShortcutCardClass = (shortcutId: string) => {
  if (invalidDragOverId.value === shortcutId) {
    return 'border-error bg-error/10'
  }

  if (dragOverId.value === shortcutId) {
    return 'border-primary bg-primary/5'
  }

  if (isManageMode.value && isShortcutSelected(shortcutId)) {
    return 'border-primary bg-primary/10'
  }

  return 'border-base-300'
}

const handleDragStart = (shortcutId: string) => {
  if (isManageMode.value) {
    return
  }

  draggedId.value = shortcutId
  dragOverId.value = shortcutId
  invalidDragOverId.value = null
}

const handleDragEnter = (shortcutId: string) => {
  if (isManageMode.value) {
    return
  }

  if (!draggedId.value || draggedId.value === shortcutId) {
    dragOverId.value = null
    invalidDragOverId.value = null
    return
  }

  if (shortcutGroupKeyById.value.get(draggedId.value) !== shortcutGroupKeyById.value.get(shortcutId)) {
    dragOverId.value = null
    invalidDragOverId.value = shortcutId
    return
  }

  invalidDragOverId.value = null
  dragOverId.value = shortcutId
}

const handleDrop = (shortcutId: string) => {
  if (isManageMode.value) {
    return
  }

  if (
    draggedId.value
    && draggedId.value !== shortcutId
    && shortcutGroupKeyById.value.get(draggedId.value) === shortcutGroupKeyById.value.get(shortcutId)
  ) {
    emit('reorder', draggedId.value, shortcutId)
  }

  resetDragState()
}

const resetDragState = () => {
  draggedId.value = null
  dragOverId.value = null
  invalidDragOverId.value = null
}

const handleToggleGroupCollapsed = (groupKey: PinnedSearchShortcutGroupKey) => {
  toggleGroupCollapsed(groupKey)
}
</script>
