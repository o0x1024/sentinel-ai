<template>
  <div
    v-if="visible"
    class="mx-4 mt-2 rounded-xl border border-info/25 bg-info/5"
  >
    <button
      type="button"
      class="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-info/10 transition-colors rounded-xl"
      @click="expanded = !expanded"
    >
      <i :class="['fas text-info text-xs', expanded ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
      <div class="w-8 h-8 rounded-full bg-info text-info-content flex items-center justify-center">
        <i class="fas fa-memory text-sm"></i>
      </div>
      <div class="flex-1 min-w-0">
        <div class="font-semibold text-sm text-info">
          {{ t('agent.retrievedMemoryTitle') }}
        </div>
        <div class="text-xs text-base-content/70 truncate">
          {{ summaryText }}
        </div>
      </div>
      <span class="badge badge-info badge-sm whitespace-nowrap">
        {{ memoryRetrieval?.hitCount ?? retrievalIds.length }} hits
      </span>
    </button>

    <div v-show="expanded" class="px-4 pb-4 space-y-3 border-t border-info/20">
      <div v-if="loading" class="flex items-center gap-2 text-sm text-base-content/70 py-2">
        <span class="loading loading-spinner loading-sm"></span>
        <span>{{ t('agent.retrievedMemoryLoading') }}</span>
      </div>
      <div v-else-if="error" class="rounded-lg border border-error/20 bg-error/10 px-3 py-2 text-sm text-error">
        {{ error }}
      </div>

      <div v-if="memoryRetrieval" class="grid gap-2 md:grid-cols-2">
        <div class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="text-[11px] uppercase tracking-wide text-base-content/50">Query</div>
          <div class="mt-1 text-sm break-words">{{ memoryRetrieval.queryPreview || '-' }}</div>
        </div>
        <div class="rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-xs space-y-1">
          <div>Tokens: ~{{ retrievalTokens }}</div>
          <div>
            Mode:
            {{ memoryRetrieval.usedCanonicalFallback ? 'canonical fallback' : 'hybrid retrieval' }}
          </div>
          <div v-if="memoryRetrieval.sourceBreakdown.length">
            Sources:
            {{ formatBreakdown(memoryRetrieval.sourceBreakdown) }}
          </div>
        </div>
      </div>

      <div v-if="items.length === 0 && !loading" class="text-sm text-base-content/60">
        {{ t('agent.retrievedMemoryEmpty') }}
      </div>

      <article
        v-for="item in items"
        :key="item.id"
        class="rounded-xl border border-base-300/70 bg-base-100 px-3 py-3"
      >
        <div class="flex flex-wrap items-center gap-2 text-xs mb-2">
          <span class="badge badge-info badge-sm">{{ item.kind }}</span>
          <span class="badge badge-ghost badge-sm">{{ item.scope }}</span>
          <span class="badge badge-ghost badge-sm">{{ item.source }}</span>
          <span
            v-if="item.autoInjectEnabled === false"
            class="badge badge-warning badge-sm"
          >
            {{ t('agent.retrievedMemoryAutoInjectDisabled') }}
          </span>
        </div>
        <div class="text-sm whitespace-pre-wrap break-words">{{ item.text }}</div>
        <div class="mt-3 flex flex-wrap justify-end gap-2">
          <button class="btn btn-xs btn-outline" @click="openInTools(item.id)">
            {{ t('agent.retrievedMemoryOpenInTools') }}
          </button>
          <button class="btn btn-xs btn-outline btn-warning" @click="toggleAutoInject(item)">
            {{
              item.autoInjectEnabled === false
                ? t('agent.retrievedMemoryEnableAutoInject')
                : t('agent.retrievedMemoryDisableAutoInject')
            }}
          </button>
          <button class="btn btn-xs btn-outline" @click="openEdit(item)">
            {{ t('agent.edit') }}
          </button>
          <button class="btn btn-xs btn-outline btn-error" @click="removeItem(item)">
            {{ t('agent.delete') }}
          </button>
        </div>
      </article>
    </div>

    <AppDialog :class="['modal', { 'modal-open': editingItem !== null }]">
      <div class="modal-box max-w-3xl" v-if="editingItem">
        <h3 class="font-bold text-lg mb-4">{{ t('agent.retrievedMemoryEditTitle') }}</h3>
        <div class="space-y-3">
          <label class="form-control">
            <div class="label py-1"><span class="label-text">{{ t('agent.retrievedMemoryEditKind') }}</span></div>
            <input v-model="editKind" class="input input-bordered input-sm" />
          </label>
          <label class="form-control">
            <div class="label py-1"><span class="label-text">{{ t('agent.retrievedMemoryEditTitleField') }}</span></div>
            <input v-model="editTitle" class="input input-bordered input-sm" />
          </label>
          <label class="form-control">
            <div class="label py-1"><span class="label-text">{{ t('agent.retrievedMemoryEditText') }}</span></div>
            <textarea v-model="editText" class="textarea textarea-bordered min-h-40 font-mono text-sm"></textarea>
          </label>
        </div>
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeEdit">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary" :disabled="saving" @click="saveEdit">
            {{ saving ? t('common.saving') : t('common.save') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop bg-black/50" @click="closeEdit">
        <button>close</button>
      </form>
    </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import AppDialog from '@/components/AppDialog.vue'
import { dialog } from '@/composables/useDialog'
import type { ContextUsageInfo, MemoryTraceCount } from '@/composables/useAgentEventTypes'
import {
  deleteDurableMemory,
  setDurableMemoryAutoInject,
  updateDurableMemory,
  useRetrievedMemoryPanel,
  type RetrievedMemoryItemView,
} from '@/composables/useRetrievedMemory'

const props = defineProps<{
  contextUsage: ContextUsageInfo | null
  visible?: boolean
}>()

const emit = defineEmits<{
  refreshed: []
}>()

const { t } = useI18n()
const router = useRouter()

const {
  items,
  loading,
  error,
  expanded,
  memoryRetrieval,
  retrievalIds,
  retrievalTokens,
  hasRetrievedMemory,
  reload,
} = useRetrievedMemoryPanel(computed(() => props.contextUsage))

const visible = computed(() => props.visible !== false && hasRetrievedMemory.value)
const saving = ref(false)
const editingItem = ref<RetrievedMemoryItemView | null>(null)
const editText = ref('')
const editTitle = ref('')
const editKind = ref('')

const summaryText = computed(() => {
  const hits = memoryRetrieval.value?.hitCount ?? retrievalIds.value.length
  const tokens = retrievalTokens.value
  return t('agent.retrievedMemorySummary', { hits, tokens })
})

const formatBreakdown = (entries: MemoryTraceCount[]) =>
  entries.map(entry => `${entry.label}:${entry.count}`).join(', ')

const openInTools = (memoryId: string) => {
  void router.push({ path: '/tools', query: { memoryId } })
}

const toggleAutoInject = async (item: RetrievedMemoryItemView) => {
  const enabled = item.autoInjectEnabled === false
  try {
    await setDurableMemoryAutoInject(item.id, enabled)
    await reload()
    emit('refreshed')
  } catch (toggleError) {
    await dialog.alert({
      message: toggleError instanceof Error ? toggleError.message : String(toggleError),
      title: t('common.error'),
    })
  }
}

const openEdit = (item: RetrievedMemoryItemView) => {
  editingItem.value = item
  editText.value = item.text
  editTitle.value = item.title || ''
  editKind.value = item.kind
}

const closeEdit = () => {
  editingItem.value = null
}

const saveEdit = async () => {
  if (!editingItem.value) return
  saving.value = true
  try {
    await updateDurableMemory({
      memoryId: editingItem.value.id,
      text: editText.value,
      title: editTitle.value,
      kind: editKind.value,
    })
    closeEdit()
    await reload()
    emit('refreshed')
  } catch (saveError) {
    await dialog.alert({
      message: saveError instanceof Error ? saveError.message : String(saveError),
      title: t('common.error'),
    })
  } finally {
    saving.value = false
  }
}

const removeItem = async (item: RetrievedMemoryItemView) => {
  const confirmed = await dialog.confirm({
    message: t('agent.retrievedMemoryDeleteConfirm'),
    title: t('agent.delete'),
  })
  if (!confirmed) return

  try {
    await deleteDurableMemory(item.id)
    await reload()
    emit('refreshed')
  } catch (deleteError) {
    await dialog.alert({
      message: deleteError instanceof Error ? deleteError.message : String(deleteError),
      title: t('common.error'),
    })
  }
}
</script>
