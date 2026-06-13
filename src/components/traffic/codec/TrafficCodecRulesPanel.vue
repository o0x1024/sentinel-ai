<template>
  <div class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h2 class="text-lg font-semibold">{{ $t('trafficAnalysis.codec.rulesPanel.title') }}</h2>
      <label class="flex items-center gap-2 text-sm">
        <span class="text-base-content/70">{{ $t('trafficAnalysis.codec.rulesPanel.globalToggle') }}:</span>
        <input
          :checked="codec.codecViewEnabled.value"
          type="checkbox"
          class="toggle toggle-sm toggle-primary"
          @change="toggleGlobalCodec"
        />
        <span>{{ codec.codecViewEnabled.value ? $t('trafficAnalysis.codec.rulesPanel.enabled') : $t('trafficAnalysis.codec.rulesPanel.disabled') }}</span>
      </label>
    </div>

    <div class="flex flex-wrap gap-2">
      <button class="btn btn-sm btn-primary" type="button" @click="openCreateDialog">
        <i class="fas fa-plus mr-1"></i>
        {{ $t('trafficAnalysis.codec.rulesPanel.newRule') }}
      </button>
      <button class="btn btn-sm btn-outline" type="button" @click="openImportDialog">
        {{ $t('trafficAnalysis.codec.rulesPanel.importRules') }}
      </button>
      <button
        class="btn btn-sm btn-outline"
        type="button"
        :disabled="exporting || sortedRules.length === 0"
        @click="exportSelected"
      >
        {{ exporting ? $t('trafficAnalysis.codec.rulesPanel.exporting') : $t('trafficAnalysis.codec.rulesPanel.exportSelected') }}
      </button>
    </div>

    <div v-if="store.loading.value" class="flex items-center justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <div
      v-else-if="sortedRules.length === 0"
      class="rounded-lg border border-dashed border-base-300 py-12 text-center text-base-content/60"
    >
      {{ $t('trafficAnalysis.codec.rulesPanel.emptyState') }}
    </div>

    <div v-else class="overflow-hidden rounded-lg border border-base-300">
      <div
        v-for="rule in sortedRules"
        :key="rule.id"
        class="flex flex-wrap items-center gap-3 border-b border-base-300 bg-base-100 px-4 py-3 last:border-b-0"
        :class="dragOverRuleId === rule.id ? 'ring-2 ring-inset ring-primary/40' : ''"
        draggable="true"
        @dragstart="handleDragStart(rule.id)"
        @dragenter.prevent="handleDragEnter(rule.id)"
        @dragover.prevent="handleDragEnter(rule.id)"
        @drop.prevent="handleDrop(rule.id)"
        @dragend="resetDragState"
      >
        <input
          :checked="selectedIds.has(rule.id)"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="toggleSelection(rule.id, ($event.target as HTMLInputElement).checked)"
        />

        <button
          type="button"
          class="btn btn-ghost btn-xs cursor-grab px-2 active:cursor-grabbing"
          tabindex="-1"
          :aria-label="$t('trafficAnalysis.codec.pipelineEditor.dragSort')"
        >
          <i class="fas fa-grip-lines text-base-content/40"></i>
        </button>

        <label class="flex items-center gap-2">
          <input
            :checked="rule.enabled"
            type="checkbox"
            class="toggle toggle-sm toggle-success"
            :disabled="togglingRuleId === rule.id"
            @change="toggleRuleEnabled(rule)"
          />
        </label>

        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
            <span class="font-medium" :class="rule.enabled ? '' : 'text-base-content/50'">
              {{ rule.name }}
            </span>
            <span class="text-sm text-base-content/60">{{ formatMatchSummary(rule) }}</span>
            <span class="badge badge-ghost badge-sm font-mono">{{ formatCodecSummary(rule) }}</span>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <button class="btn btn-ghost btn-xs" type="button" @click="openEditDialog(rule)">
            {{ $t('trafficAnalysis.codec.rulesPanel.edit') }}
          </button>
          <button
            class="btn btn-ghost btn-xs text-error"
            type="button"
            :disabled="deletingRuleId === rule.id"
            @click="confirmDelete(rule)"
          >
            {{ $t('trafficAnalysis.codec.rulesPanel.delete') }}
          </button>
        </div>
      </div>
    </div>

    <TrafficCodecRuleDialog
      ref="ruleDialogRef"
      :edit-rule="editingRule"
      @saved="handleRuleSaved"
      @closed="handleRuleDialogClosed"
    />

    <TrafficCodecImportExportDialog
      ref="importDialogRef"
      @success="handleImportSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import { toast } from '@/composables/useToast'
import TrafficCodecImportExportDialog from './TrafficCodecImportExportDialog.vue'
import TrafficCodecRuleDialog from './TrafficCodecRuleDialog.vue'
import { downloadAsFile, exportCodecRules } from './trafficCodecExportImport'
import { useTrafficCodecRuleStore } from './trafficCodecRuleStore'
import { useTrafficCodec } from './useTrafficCodec'
import { BUILTIN_CODECS, type TrafficCodecRule } from './trafficCodecTypes'

const { t } = useI18n()
const store = useTrafficCodecRuleStore()
const codec = useTrafficCodec()

const ruleDialogRef = ref<InstanceType<typeof TrafficCodecRuleDialog> | null>(null)
const importDialogRef = ref<InstanceType<typeof TrafficCodecImportExportDialog> | null>(null)
const editingRule = ref<TrafficCodecRule | undefined>(undefined)
const selectedIds = ref(new Set<string>())
const exporting = ref(false)
const deletingRuleId = ref<string | null>(null)
const togglingRuleId = ref<string | null>(null)
const dragRuleId = ref<string | null>(null)
const dragOverRuleId = ref<string | null>(null)

const sortedRules = computed(() =>
  [...store.rules.value].sort((a, b) => a.order - b.order),
)

const codecLabelById = new Map<string, string>(
  BUILTIN_CODECS.map(item => [item.id, item.label]),
)

onMounted(() => {
  store.loadRules()
})

function formatMatchSummary(rule: TrafficCodecRule): string {
  const hosts = rule.matchRule.hosts.length > 0 ? rule.matchRule.hosts.join(', ') : '*'
  const paths = rule.matchRule.paths.length > 0 ? rule.matchRule.paths.join(', ') : '*'
  return `${hosts}  ${paths}`
}

function formatCodecSummary(rule: TrafficCodecRule): string {
  const enabledSteps = rule.pipeline.steps.filter(step => step.enabled)
  if (enabledSteps.length === 0) return t('trafficAnalysis.codec.rulesPanel.noSteps')
  return enabledSteps
    .map(step => codecLabelById.get(step.codec) ?? step.codec.toUpperCase())
    .join(' → ')
}

function toggleSelection(ruleId: string, checked: boolean) {
  const next = new Set(selectedIds.value)
  if (checked) {
    next.add(ruleId)
  } else {
    next.delete(ruleId)
  }
  selectedIds.value = next
}

function toggleGlobalCodec() {
  codec.codecViewEnabled.value = !codec.codecViewEnabled.value
  codec.invalidateCache()
}

function openCreateDialog() {
  editingRule.value = undefined
  ruleDialogRef.value?.showModal()
}

function openEditDialog(rule: TrafficCodecRule) {
  editingRule.value = rule
  ruleDialogRef.value?.showModal()
}

function handleRuleDialogClosed() {
  editingRule.value = undefined
}

function handleRuleSaved() {
  codec.invalidateCache()
}

function openImportDialog() {
  importDialogRef.value?.showModal()
}

async function handleImportSuccess() {
  await store.loadRules()
  codec.invalidateCache()
}

async function toggleRuleEnabled(rule: TrafficCodecRule) {
  togglingRuleId.value = rule.id
  try {
    await store.saveRule({
      ...rule,
      enabled: !rule.enabled,
      updatedAt: new Date().toISOString(),
    })
    codec.invalidateCache()
  } finally {
    togglingRuleId.value = null
  }
}

async function confirmDelete(rule: TrafficCodecRule) {
  const confirmed = await dialog.confirm(t('trafficAnalysis.codec.rulesPanel.deleteConfirm', { name: rule.name }))
  if (!confirmed) return

  deletingRuleId.value = rule.id
  try {
    await store.deleteRule(rule.id)
    const nextSelected = new Set(selectedIds.value)
    nextSelected.delete(rule.id)
    selectedIds.value = nextSelected
    codec.invalidateCache()
  } finally {
    deletingRuleId.value = null
  }
}

async function exportSelected() {
  const ruleIds =
    selectedIds.value.size > 0
      ? [...selectedIds.value]
      : sortedRules.value.map(rule => rule.id)

  exporting.value = true
  try {
    const json = await exportCodecRules(ruleIds)
    const timestamp = new Date().toISOString().slice(0, 10)
    downloadAsFile(json, `codec-rules-${timestamp}.json`)
  } catch (error) {
    toast.error(error instanceof Error ? error.message : t('trafficAnalysis.codec.rulesPanel.exportFailed'))
  } finally {
    exporting.value = false
  }
}

function handleDragStart(ruleId: string) {
  dragRuleId.value = ruleId
}

function handleDragEnter(ruleId: string) {
  dragOverRuleId.value = ruleId
}

async function handleDrop(targetRuleId: string) {
  if (!dragRuleId.value || dragRuleId.value === targetRuleId) {
    resetDragState()
    return
  }

  const ids = sortedRules.value.map(rule => rule.id)
  const fromIndex = ids.indexOf(dragRuleId.value)
  const toIndex = ids.indexOf(targetRuleId)
  if (fromIndex < 0 || toIndex < 0) {
    resetDragState()
    return
  }

  const [moved] = ids.splice(fromIndex, 1)
  ids.splice(toIndex, 0, moved)

  resetDragState()
  await store.reorderRules(ids)
  codec.invalidateCache()
}

function resetDragState() {
  dragRuleId.value = null
  dragOverRuleId.value = null
}
</script>
