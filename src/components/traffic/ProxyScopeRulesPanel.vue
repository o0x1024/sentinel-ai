<template>
  <div class="space-y-8">
    <div>
      <h3 class="font-medium">
        {{ $t('trafficAnalysis.proxyConfiguration.trafficScopeRules') }}
      </h3>
      <p class="text-xs text-base-content/60 mt-1">
        {{ $t('trafficAnalysis.proxyConfiguration.trafficScopeRulesDesc') }}
      </p>
    </div>

    <div class="space-y-8">
      <section class="space-y-3">
        <div class="font-medium text-lg">
          {{ $t('trafficAnalysis.proxyConfiguration.scopeIncludeTitle') }}
        </div>
        <div class="flex gap-3">
          <div class="flex flex-col gap-2 shrink-0 w-32">
            <button class="btn btn-sm scope-side-btn" type="button" @click="openCreateDialog('include')">
              {{ $t('trafficAnalysis.proxyConfiguration.add') }}
            </button>
            <button
              class="btn btn-sm scope-side-btn"
              type="button"
              :disabled="selectedIncludeIndex === -1"
              @click="openEditDialog('include')"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.edit') }}
            </button>
            <button
              class="btn btn-sm scope-side-btn"
              type="button"
              :disabled="selectedIncludeIndex === -1"
              @click="removeRule('include')"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.remove') }}
            </button>
            <button class="btn btn-sm scope-side-btn" type="button" @click="pasteUrl('include')">
              {{ $t('trafficAnalysis.proxyConfiguration.pasteUrl') }}
            </button>
            <button class="btn btn-sm scope-side-btn" type="button" @click="triggerLoad('include')">
              {{ $t('trafficAnalysis.proxyConfiguration.loadScopeRules') }}
            </button>
          </div>

          <div class="flex-1 overflow-x-auto border border-base-300 bg-base-100">
            <table class="table table-sm w-full scope-table">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enabled') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.protocol') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.scopeHostOrIpRange') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.port') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.file') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(rule, index) in includeRules"
                  :key="`include-${index}`"
                  class="cursor-pointer hover:bg-base-200/70"
                  :class="{ 'bg-primary/10': selectedIncludeIndex === index }"
                  @click="selectedIncludeIndex = index"
                  @dblclick="openEditDialog('include')"
                >
                  <td>
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      :checked="rule.enabled"
                      @click.stop="toggleRuleEnabled('include', index)"
                    />
                  </td>
                  <td>{{ getProtocolLabel(rule.protocol) }}</td>
                  <td class="font-mono text-xs">{{ rule.host_or_ip_range || '*' }}</td>
                  <td class="font-mono text-xs">{{ rule.port || '*' }}</td>
                  <td class="font-mono text-xs">{{ rule.file || '*' }}</td>
                </tr>
                <tr v-if="includeRules.length === 0">
                  <td colspan="5" class="text-center text-sm text-base-content/50 py-6">
                    {{ $t('trafficAnalysis.proxyConfiguration.scopeIncludeEmpty') }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <section class="space-y-3">
        <div class="font-medium text-lg">
          {{ $t('trafficAnalysis.proxyConfiguration.scopeExcludeTitle') }}
        </div>
        <div class="flex gap-3">
          <div class="flex flex-col gap-2 shrink-0 w-32">
            <button class="btn btn-sm scope-side-btn" type="button" @click="openCreateDialog('exclude')">
              {{ $t('trafficAnalysis.proxyConfiguration.add') }}
            </button>
            <button
              class="btn btn-sm scope-side-btn"
              type="button"
              :disabled="selectedExcludeIndex === -1"
              @click="openEditDialog('exclude')"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.edit') }}
            </button>
            <button
              class="btn btn-sm scope-side-btn"
              type="button"
              :disabled="selectedExcludeIndex === -1"
              @click="removeRule('exclude')"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.remove') }}
            </button>
            <button class="btn btn-sm scope-side-btn" type="button" @click="pasteUrl('exclude')">
              {{ $t('trafficAnalysis.proxyConfiguration.pasteUrl') }}
            </button>
            <button class="btn btn-sm scope-side-btn" type="button" @click="triggerLoad('exclude')">
              {{ $t('trafficAnalysis.proxyConfiguration.loadScopeRules') }}
            </button>
          </div>

          <div class="flex-1 overflow-x-auto border border-base-300 bg-base-100">
            <table class="table table-sm w-full scope-table">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enabled') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.protocol') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.scopeHostOrIpRange') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.port') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.file') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(rule, index) in excludeRules"
                  :key="`exclude-${index}`"
                  class="cursor-pointer hover:bg-base-200/70"
                  :class="{ 'bg-primary/10': selectedExcludeIndex === index }"
                  @click="selectedExcludeIndex = index"
                  @dblclick="openEditDialog('exclude')"
                >
                  <td>
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      :checked="rule.enabled"
                      @click.stop="toggleRuleEnabled('exclude', index)"
                    />
                  </td>
                  <td>{{ getProtocolLabel(rule.protocol) }}</td>
                  <td class="font-mono text-xs">{{ rule.host_or_ip_range || '*' }}</td>
                  <td class="font-mono text-xs">{{ rule.port || '*' }}</td>
                  <td class="font-mono text-xs">{{ rule.file || '*' }}</td>
                </tr>
                <tr v-if="excludeRules.length === 0">
                  <td colspan="5" class="text-center text-sm text-base-content/50 py-6">
                    {{ $t('trafficAnalysis.proxyConfiguration.scopeExcludeEmpty') }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </section>
    </div>

    <AppDialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-2xl px-0 py-0 overflow-hidden">
        <div class="border-b border-base-300 px-5 py-4">
          <h3 class="text-xl font-semibold text-center">
            {{
              dialogMode === 'include'
                ? $t(editingIndex === -1
                  ? 'trafficAnalysis.proxyConfiguration.addScopeIncludeRule'
                  : 'trafficAnalysis.proxyConfiguration.editScopeIncludeRule')
                : $t(editingIndex === -1
                  ? 'trafficAnalysis.proxyConfiguration.addScopeExcludeRule'
                  : 'trafficAnalysis.proxyConfiguration.editScopeExcludeRule')
            }}
          </h3>
        </div>

        <div class="px-5 py-5">
          <div class="flex items-start gap-3 mb-5">
            <div class="shrink-0 text-base-content/70 pt-0.5">
              <i class="far fa-question-circle text-2xl"></i>
            </div>
            <p class="text-sm leading-6 text-base-content/85">
              {{ $t('trafficAnalysis.proxyConfiguration.scopeRuleDialogDesc') }}
            </p>
          </div>

          <div class="space-y-3">
            <label class="label cursor-pointer justify-start gap-3 pl-[156px] py-0">
              <input v-model="editingRule.enabled" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.enabled') }}</span>
            </label>

            <div class="scope-form-row">
              <label class="scope-form-label">
                {{ $t('trafficAnalysis.proxyConfiguration.protocol') }}:
              </label>
              <select v-model="editingRule.protocol" class="select select-bordered w-full">
                <option v-for="item in protocolOptions" :key="item.value" :value="item.value">
                  {{ item.label }}
                </option>
              </select>
            </div>

            <div class="scope-form-row">
              <label class="scope-form-label">
                {{ $t('trafficAnalysis.proxyConfiguration.scopeHostOrIpRange') }}:
              </label>
              <input
                v-model="editingRule.host_or_ip_range"
                type="text"
                class="input input-bordered w-full"
                :placeholder="$t('trafficAnalysis.proxyConfiguration.scopeHostOrIpRangePlaceholder')"
              />
            </div>

            <div class="scope-form-row">
              <label class="scope-form-label">
                {{ $t('trafficAnalysis.proxyConfiguration.port') }}:
              </label>
              <input
                v-model="editingRule.port"
                type="text"
                class="input input-bordered w-full"
                :placeholder="$t('trafficAnalysis.proxyConfiguration.scopePortPlaceholder')"
              />
            </div>

            <div class="scope-form-row">
              <label class="scope-form-label">
                {{ $t('trafficAnalysis.proxyConfiguration.file') }}:
              </label>
              <input
                v-model="editingRule.file"
                type="text"
                class="input input-bordered w-full"
                :placeholder="$t('trafficAnalysis.proxyConfiguration.scopeFilePlaceholder')"
              />
            </div>
          </div>
        </div>

        <div class="flex items-center justify-between border-t border-base-300 px-5 py-4 bg-base-100">
          <button
            v-if="!dialogOpenedFromPaste"
            class="btn btn-md min-w-32"
            type="button"
            @click="pasteUrl(dialogMode)"
          >
            {{ $t('trafficAnalysis.proxyConfiguration.pasteUrl') }}
          </button>
          <div v-else></div>
          <div class="flex items-center gap-3">
            <button class="btn btn-md min-w-28" type="button" @click="saveDialog">
              {{ $t('trafficAnalysis.proxyConfiguration.ok') }}
            </button>
            <button class="btn btn-md min-w-28 btn-ghost" type="button" @click="closeDialog">
              {{ $t('trafficAnalysis.proxyConfiguration.cancel') }}
            </button>
          </div>
        </div>
      </div>
    </AppDialog>

    <input
      ref="includeFileInputRef"
      type="file"
      class="hidden"
      accept=".json,.txt,.scope"
      @change="handleFileChange('include', $event)"
    />
    <input
      ref="excludeFileInputRef"
      type="file"
      class="hidden"
      accept=".json,.txt,.scope"
      @change="handleFileChange('exclude', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import { createDefaultProxyScopeRule, type ProxyScopeRule } from './proxyConfigurationTypes'
import { buildProxyScopeRuleFromUrl } from './proxyScopeRuleUrlSupport'

type ScopeListType = 'include' | 'exclude'

const props = defineProps<{
  includeRules: ProxyScopeRule[]
  excludeRules: ProxyScopeRule[]
}>()

const emit = defineEmits<{
  (e: 'update:includeRules', value: ProxyScopeRule[]): void
  (e: 'update:excludeRules', value: ProxyScopeRule[]): void
}>()

const { t } = useI18n()

const dialogRef = ref<HTMLDialogElement | null>(null)
const dialogMode = ref<ScopeListType>('include')
const editingIndex = ref(-1)
const editingRule = ref<ProxyScopeRule>(createDefaultProxyScopeRule())
const dialogOpenedFromPaste = ref(false)
const selectedIncludeIndex = ref(-1)
const selectedExcludeIndex = ref(-1)
const includeFileInputRef = ref<HTMLInputElement | null>(null)
const excludeFileInputRef = ref<HTMLInputElement | null>(null)

const protocolOptions = computed(() => [
  { value: 'any', label: t('trafficAnalysis.proxyConfiguration.scopeProtocolAny') },
  { value: 'http', label: 'HTTP' },
  { value: 'https', label: 'HTTPS' },
  { value: 'ws', label: 'WS' },
  { value: 'wss', label: 'WSS' },
])

function cloneRule(rule: ProxyScopeRule): ProxyScopeRule {
  return {
    enabled: rule.enabled,
    protocol: rule.protocol,
    host_or_ip_range: rule.host_or_ip_range,
    port: rule.port,
    file: rule.file,
  }
}

function getRules(type: ScopeListType): ProxyScopeRule[] {
  return type === 'include' ? props.includeRules : props.excludeRules
}

function emitRules(type: ScopeListType, rules: ProxyScopeRule[]) {
  if (type === 'include') {
    emit('update:includeRules', rules)
    return
  }
  emit('update:excludeRules', rules)
}

function normalizeImportedRule(rule: Partial<ProxyScopeRule>): ProxyScopeRule {
  return {
    ...createDefaultProxyScopeRule(),
    ...rule,
  }
}

function openCreateDialog(type: ScopeListType) {
  dialogMode.value = type
  editingIndex.value = -1
  dialogOpenedFromPaste.value = false
  editingRule.value = createDefaultProxyScopeRule()
  dialogRef.value?.showModal()
}

function openEditDialog(type: ScopeListType) {
  const index = type === 'include' ? selectedIncludeIndex.value : selectedExcludeIndex.value
  if (index === -1) {
    return
  }
  dialogMode.value = type
  editingIndex.value = index
  dialogOpenedFromPaste.value = false
  editingRule.value = cloneRule(getRules(type)[index])
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogOpenedFromPaste.value = false
  dialogRef.value?.close()
}

function saveDialog() {
  const nextRules = [...getRules(dialogMode.value)]
  const normalizedRule = cloneRule(editingRule.value)
  if (editingIndex.value === -1) {
    nextRules.push(normalizedRule)
  } else {
    nextRules.splice(editingIndex.value, 1, normalizedRule)
  }
  emitRules(dialogMode.value, nextRules)
  closeDialog()
}

function removeRule(type: ScopeListType) {
  const selectedIndex = type === 'include' ? selectedIncludeIndex.value : selectedExcludeIndex.value
  if (selectedIndex === -1) {
    return
  }
  const nextRules = [...getRules(type)]
  nextRules.splice(selectedIndex, 1)
  emitRules(type, nextRules)
  if (type === 'include') {
    selectedIncludeIndex.value = -1
  } else {
    selectedExcludeIndex.value = -1
  }
}

function toggleRuleEnabled(type: ScopeListType, index: number) {
  const nextRules = [...getRules(type)]
  nextRules[index] = {
    ...nextRules[index],
    enabled: !nextRules[index].enabled,
  }
  emitRules(type, nextRules)
}

function getProtocolLabel(value: string) {
  const normalized = (value || 'any').toLowerCase()
  return protocolOptions.value.find(item => item.value === normalized)?.label || value
}

async function pasteUrl(type: ScopeListType) {
  try {
    const response = await invoke<{ success: boolean; data: string; error?: string }>(
      'read_traffic_clipboard_text',
    )
    if (!response?.success) {
      throw new Error(response?.error || 'Failed to read clipboard text')
    }
    const text = (response.data || '').trim()
    if (!text) {
      return
    }
    const parsed = new URL(text)
    dialogMode.value = type
    editingIndex.value = -1
    dialogOpenedFromPaste.value = true
    editingRule.value = buildProxyScopeRuleFromUrl(parsed)
    dialogRef.value?.showModal()
  } catch (error) {
    console.error('[ProxyScopeRulesPanel] Failed to paste scope URL:', error)
    dialog.toast.error(t('trafficAnalysis.proxyConfiguration.pasteUrlFailed'))
  }
}

function triggerLoad(type: ScopeListType) {
  const input = type === 'include' ? includeFileInputRef.value : excludeFileInputRef.value
  input?.click()
}

function parseScopeRulesFromText(type: ScopeListType, text: string): ProxyScopeRule[] {
  const trimmed = text.trim()
  if (!trimmed) {
    return []
  }

  try {
    const parsed = JSON.parse(trimmed)
    if (Array.isArray(parsed)) {
      return parsed.map(rule => normalizeImportedRule(rule as Partial<ProxyScopeRule>))
    }
    if (parsed && typeof parsed === 'object') {
      const key = type === 'include' ? 'includeRules' : 'excludeRules'
      const rules = (parsed as Record<string, unknown>)[key]
      if (Array.isArray(rules)) {
        return rules.map(rule => normalizeImportedRule(rule as Partial<ProxyScopeRule>))
      }
    }
  } catch {
    // Fallback to plain text parsing below.
  }

  return trimmed
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(Boolean)
    .map(line => {
      try {
        const parsed = new URL(line)
        return normalizeImportedRule(buildProxyScopeRuleFromUrl(parsed))
      } catch {
        return normalizeImportedRule({
          host_or_ip_range: line,
        })
      }
    })
}

async function handleFileChange(type: ScopeListType, event: Event) {
  const input = event.target as HTMLInputElement | null
  const file = input?.files?.[0]
  if (!file) {
    return
  }

  try {
    const content = await file.text()
    const rules = parseScopeRulesFromText(type, content)
    emitRules(type, rules)
    if (type === 'include') {
      selectedIncludeIndex.value = -1
    } else {
      selectedExcludeIndex.value = -1
    }
    dialog.toast.success(
      t('trafficAnalysis.proxyConfiguration.loadScopeRulesSuccess', {
        count: rules.length,
      }),
    )
  } catch (error) {
    console.error('[ProxyScopeRulesPanel] Failed to load scope rules:', error)
    dialog.toast.error(t('trafficAnalysis.proxyConfiguration.loadScopeRulesFailed'))
  } finally {
    if (input) {
      input.value = ''
    }
  }
}
</script>

<style scoped>
.scope-side-btn {
  justify-content: center;
  border-color: oklch(var(--b3));
  background: oklch(var(--b2));
  font-weight: 600;
}

.scope-side-btn:hover {
  background: oklch(var(--b3));
}

.scope-table :deep(thead th) {
  background: oklch(var(--b2));
  font-weight: 600;
  border-bottom: 1px solid oklch(var(--b3));
}

.scope-table :deep(td),
.scope-table :deep(th) {
  white-space: nowrap;
}

.scope-form-row {
  display: grid;
  grid-template-columns: 140px minmax(0, 1fr);
  align-items: center;
  gap: 16px;
}

.scope-form-label {
  font-size: 0.95rem;
  font-weight: 500;
  text-align: right;
  color: oklch(var(--bc));
}

@media (max-width: 768px) {
  .scope-form-row {
    grid-template-columns: 1fr;
    gap: 10px;
  }

  .scope-form-label {
    text-align: left;
  }
}
</style>
