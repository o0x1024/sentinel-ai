<template>
  <AppDialog ref="dialogRef" class="modal">
    <div class="modal-box max-w-2xl overflow-hidden px-0 py-0">
      <div class="border-b border-base-300 px-5 py-4">
        <h3 class="text-center text-xl font-semibold">
          {{
            mode === 'include'
              ? $t('trafficAnalysis.proxyConfiguration.addScopeIncludeRule')
              : $t('trafficAnalysis.proxyConfiguration.addScopeExcludeRule')
          }}
        </h3>
      </div>

      <div class="px-5 py-5">
        <div class="mb-5 flex items-start gap-3">
          <div class="shrink-0 pt-0.5 text-base-content/70">
            <i class="far fa-question-circle text-2xl"></i>
          </div>
          <p class="text-sm leading-6 text-base-content/85">
            {{ $t('trafficAnalysis.proxyConfiguration.scopeRuleDialogDesc') }}
          </p>
        </div>

        <div class="space-y-3">
          <label class="label cursor-pointer justify-start gap-3 py-0 pl-[156px]">
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

      <div class="flex items-center justify-end gap-3 border-t border-base-300 bg-base-100 px-5 py-4">
        <button class="btn btn-md min-w-28" type="button" @click="save">
          {{ $t('trafficAnalysis.proxyConfiguration.ok') }}
        </button>
        <button class="btn btn-md min-w-28 btn-ghost" type="button" @click="close">
          {{ $t('trafficAnalysis.proxyConfiguration.cancel') }}
        </button>
      </div>
    </div>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { createDefaultProxyScopeRule } from './proxyConfigurationTypes'
import type { ProxyScopeRule } from './proxyConfigurationTypes'
import {
  cloneProxyScopeRule,
  type ScopeRuleDialogOpenPayload,
  type ScopeRuleDialogSavePayload,
  type TrafficScopeRuleMode,
} from './trafficScopeRuleActions'

const emit = defineEmits<{
  (e: 'save', payload: ScopeRuleDialogSavePayload): void
}>()

const { t } = useI18n()

const dialogRef = ref<HTMLDialogElement | null>(null)
const mode = ref<TrafficScopeRuleMode>('include')
const editingRule = ref<ProxyScopeRule>(createDefaultProxyScopeRule())

const protocolOptions = computed(() => [
  { value: 'any', label: t('trafficAnalysis.proxyConfiguration.scopeProtocolAny') },
  { value: 'http', label: 'HTTP' },
  { value: 'https', label: 'HTTPS' },
  { value: 'ws', label: 'WS' },
  { value: 'wss', label: 'WSS' },
])

function open(payload: ScopeRuleDialogOpenPayload) {
  mode.value = payload.mode
  editingRule.value = cloneProxyScopeRule(payload.rule)
  dialogRef.value?.showModal()
}

function close() {
  dialogRef.value?.close()
}

function save() {
  emit('save', {
    mode: mode.value,
    rule: cloneProxyScopeRule(editingRule.value),
  })
  close()
}

defineExpose({
  open,
})
</script>

<style scoped>
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
