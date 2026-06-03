<template>
  <AppModal
    :open="open"
    box-class="w-[84vw] max-w-[900px] max-h-[74vh] rounded-2xl border border-base-300/70 shadow-xl p-0"
    @close="$emit('close')"
  >
    <div class="filter-shell flex max-h-[74vh] min-h-0 flex-col overflow-hidden">
      <div class="border-b border-base-300/70 px-4 py-2.5">
        <h3 class="text-center text-lg font-semibold text-base-content">
          {{ t('trafficAnalysis.history.filterDialog.shellTitle', '配置 HTTP 代理过滤器') }}
        </h3>
      </div>

      <div class="border-b border-base-300/50 px-4 py-2">
        <div class="tabs filter-mode-tabs tabs-sm inline-grid grid-cols-2 gap-1 bg-base-200/80 p-1">
          <button
            type="button"
            class="tab h-auto min-h-0 px-3 py-1.5 text-sm font-medium transition-colors"
            :class="mode === 'settings' ? 'tab-active !bg-primary !text-primary-content' : 'text-base-content/75'"
            @click="mode = 'settings'"
          >
            {{ t('trafficAnalysis.history.filterDialog.settingsMode', '设置模式') }}
          </button>
          <button
            type="button"
            class="tab h-auto min-h-0 px-3 py-1.5 text-sm font-medium text-base-content/45"
            disabled
            :title="t('trafficAnalysis.history.filterDialog.bambdaUnavailable')"
          >
            {{ t('trafficAnalysis.history.filterDialog.bambdaMode', 'Bambda 模式') }}
          </button>
        </div>
      </div>

      <div v-if="mode === 'settings'" class="min-h-0 flex-1 overflow-auto bg-base-200/25 px-4 py-2.5">
        <div class="space-y-2">
          <div class="filter-grid-top">
            <fieldset class="filter-card">
              <legend class="filter-card-title px-1">{{ t('trafficAnalysis.history.filterDialog.requestTypeTitle', '按请求类型过滤') }}</legend>
              <div class="filter-card-body space-y-1.5">
                <label class="filter-checkbox-row">
                  <input v-model="draft.requestType.showOnlyInScope" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.showOnlyInScope', '仅显示目标范围内项') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.requestType.hideWithoutResponse" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.hideWithoutResponse') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.requestType.showOnlyWithParams" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.showOnlyWithParams') }}</span>
                </label>
              </div>
            </fieldset>

            <fieldset class="filter-card">
              <legend class="filter-card-title px-1">{{ t('trafficAnalysis.history.filterDialog.mimeTypeTitle', '按 MIME 类型过滤') }}</legend>
              <div class="filter-card-body grid grid-cols-2 gap-x-4 gap-y-1.5">
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.html" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.html') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.otherText" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.otherText') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.script" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.script') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.images" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.images') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.xml" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.xml') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.flash" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.flash') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.css" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.css') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.mimeType.otherBinary" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.otherBinary') }}</span>
                </label>
              </div>
            </fieldset>

            <fieldset class="filter-card">
              <legend class="filter-card-title px-1">{{ t('trafficAnalysis.history.filterDialog.statusCodeTitle', '按状态码过滤') }}</legend>
              <div class="filter-card-body grid gap-x-3 gap-y-1.5 sm:grid-cols-2">
                <label class="filter-checkbox-row">
                  <input v-model="draft.statusCode.s2xx" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.status2xx') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.statusCode.s3xx" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.status3xx') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.statusCode.s4xx" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.status4xx') }}</span>
                </label>
                <label class="filter-checkbox-row">
                  <input v-model="draft.statusCode.s5xx" type="checkbox" class="checkbox checkbox-sm" />
                  <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.status5xx') }}</span>
                </label>
              </div>
            </fieldset>
          </div>

          <div class="filter-grid-bottom">
            <fieldset class="filter-card">
              <legend class="filter-card-title px-1">{{ t('trafficAnalysis.history.filterDialog.searchTitle', '按搜索词过滤') }}</legend>
              <div class="filter-card-body space-y-1.5">
                <input
                  v-model="draft.search.term"
                  type="text"
                  class="input input-bordered input-sm h-8 w-full"
                  :placeholder="t('trafficAnalysis.history.filterDialog.checkboxes.placeholders.search')"
                />
                <div class="grid grid-cols-2 gap-x-3 gap-y-1.5 xl:grid-cols-3">
                  <label class="filter-checkbox-row">
                    <input v-model="draft.search.regex" type="checkbox" class="checkbox checkbox-sm" />
                    <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.regex') }}</span>
                  </label>
                  <label class="filter-checkbox-row">
                    <input v-model="draft.search.caseSensitive" type="checkbox" class="checkbox checkbox-sm" />
                    <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.caseSensitive') }}</span>
                  </label>
                  <label class="filter-checkbox-row col-span-2 xl:col-span-1">
                    <input v-model="draft.search.negative" type="checkbox" class="checkbox checkbox-sm" />
                    <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.negativeSearch') }}</span>
                  </label>
                </div>
              </div>
            </fieldset>

            <fieldset class="filter-card">
              <legend class="filter-card-title px-1">{{ t('trafficAnalysis.history.filterDialog.fileExtensionTitle', '按文件扩展名过滤') }}</legend>
              <div class="filter-card-body space-y-1.5">
                <div class="extension-row">
                  <label class="filter-checkbox-row">
                    <input v-model="draft.extension.showOnlyEnabled" type="checkbox" class="checkbox checkbox-sm" />
                    <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.labels.showOnly') }}</span>
                  </label>
                  <input
                    v-model="draft.extension.showOnly"
                    type="text"
                    class="input input-bordered input-sm h-8 flex-1"
                    :disabled="!draft.extension.showOnlyEnabled"
                    :placeholder="t('trafficAnalysis.history.filterDialog.checkboxes.placeholders.showExtensions')"
                  />
                </div>
                <div class="extension-row">
                  <label class="filter-checkbox-row">
                    <input v-model="draft.extension.hideEnabled" type="checkbox" class="checkbox checkbox-sm" />
                    <span>{{ t('trafficAnalysis.history.filterDialog.checkboxes.labels.hide') }}</span>
                  </label>
                  <input
                    v-model="draft.extension.hide"
                    type="text"
                    class="input input-bordered input-sm h-8 flex-1"
                    :disabled="!draft.extension.hideEnabled"
                    :placeholder="t('trafficAnalysis.history.filterDialog.checkboxes.placeholders.hideExtensions')"
                  />
                </div>
              </div>
            </fieldset>

            <fieldset class="filter-card">
              <legend class="filter-card-title px-1">{{ t('trafficAnalysis.history.filterDialog.listenerTitle', '按监听器过滤') }}</legend>
              <div class="filter-card-body">
                <div class="listener-row">
                  <label class="filter-inline-label">{{ t('trafficAnalysis.history.filterDialog.checkboxes.port') }}</label>
                  <input
                    v-model="draft.listener.port"
                    type="text"
                    class="input input-bordered input-sm h-8 w-full"
                    :placeholder="t('trafficAnalysis.history.filterDialog.checkboxes.placeholders.port')"
                  />
                </div>
              </div>
            </fieldset>
          </div>
        </div>
      </div>

      <div class="border-t border-base-300/70 bg-base-100 px-4 py-2">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="btn btn-xs btn-ghost rounded-2xl border border-base-300/80 bg-base-100 px-3 text-base-content/80 hover:border-base-400/80"
              @click="showAll"
            >
                <i class="fas fa-eye mr-1"></i>
                {{ t('trafficAnalysis.history.filterDialog.showAll', '显示全部') }}
            </button>
            <button
              type="button"
              class="btn btn-xs btn-ghost rounded-2xl border border-base-300/80 bg-base-100 px-3 text-base-content/80 hover:border-base-400/80"
              @click="hideAll"
            >
                <i class="fas fa-eye-slash mr-1"></i>
                {{ t('trafficAnalysis.history.filterDialog.hideAll', '隐藏全部') }}
            </button>
            <button
              type="button"
              class="btn btn-xs btn-ghost rounded-2xl border border-base-300/80 bg-base-100 px-3 text-base-content/80 hover:border-base-400/80"
              @click="revertChanges"
            >
                <i class="fas fa-rotate-left mr-1"></i>
                {{ t('trafficAnalysis.history.filterDialog.revertChanges', '恢复更改') }}
            </button>
          </div>

          <div class="flex items-center gap-2">
            <button type="button" class="btn btn-sm btn-ghost rounded-2xl px-3.5" @click="$emit('close')">
              {{ t('trafficAnalysis.history.filterDialog.cancel', '取消') }}
            </button>
            <button type="button" class="btn btn-sm btn-primary rounded-2xl px-3.5" @click="applyConfig">
              <i class="fas fa-check mr-1"></i>
              {{ t('trafficAnalysis.history.filterDialog.applyAndClose') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import AppModal from '@/components/AppModal.vue'

import {
  hideAllProxyHistoryFilters,
  showAllProxyHistoryFilters,
} from './proxyHistoryFilterSupport'
import type { ProxyHistoryFilterConfig } from './proxyHistoryTypes'

const props = defineProps<{
  open: boolean
  config: ProxyHistoryFilterConfig
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'apply', config: ProxyHistoryFilterConfig): void
}>()

const { t } = useI18n()
const mode = ref<'settings' | 'bambda'>('settings')
const originalConfig = ref<ProxyHistoryFilterConfig>(cloneFilterConfig(props.config))
const draft = reactive(cloneFilterConfig(props.config))

watch(
  () => [props.open, props.config] as const,
  ([open]) => {
    if (!open) {
      return
    }

    mode.value = 'settings'
    originalConfig.value = cloneFilterConfig(props.config)
    Object.assign(draft, cloneFilterConfig(props.config))
  },
  { immediate: true, deep: true },
)

function showAll() {
  Object.assign(draft, showAllProxyHistoryFilters(cloneFilterConfig(draft)))
}

function hideAll() {
  Object.assign(draft, hideAllProxyHistoryFilters(cloneFilterConfig(draft)))
}

function revertChanges() {
  Object.assign(draft, cloneFilterConfig(originalConfig.value))
}

function applyConfig() {
  emit('apply', cloneFilterConfig(draft))
}

function cloneFilterConfig(config: ProxyHistoryFilterConfig): ProxyHistoryFilterConfig {
  return JSON.parse(JSON.stringify(config)) as ProxyHistoryFilterConfig
}
</script>

<style scoped>
.filter-shell {
  background: hsl(var(--b1));
}

.filter-mode-tabs {
  border-radius: 0.35rem;
}

.filter-mode-tabs :deep(.tab) {
  border-radius: 0.25rem;
}

.filter-card {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  border-radius: 1rem;
  border: 1px solid hsl(var(--b3) / 0.64);
  background: hsl(var(--b1));
  padding: 0.75rem 0.8rem;
}

.filter-card-title {
  display: block;
  min-height: 1.1rem;
  margin-bottom: 0.4rem;
  font-size: 0.82rem;
  font-weight: 700;
  line-height: 1.2;
  color: hsl(var(--bc));
}

.filter-card-body {
  min-height: 0;
  flex: 1;
  align-content: start;
}

.filter-checkbox-row {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  min-height: 1.2rem;
  min-width: 0;
  font-size: 0.88rem;
  line-height: 1.2;
  color: hsl(var(--bc) / 0.9);
}

.filter-checkbox-row span {
  word-break: keep-all;
  overflow-wrap: break-word;
}

.extension-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 0.65rem;
}

.listener-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: 0.65rem;
}

.filter-inline-label {
  font-size: 0.86rem;
  font-weight: 600;
  color: hsl(var(--bc) / 0.84);
  white-space: nowrap;
}

.filter-grid-top,
.filter-grid-bottom {
  display: grid;
  align-items: stretch;
  gap: 0.65rem;
}

@media (min-width: 1024px) {
  .filter-grid-top > .filter-card {
    min-height: 11.2rem;
  }

  .filter-grid-bottom > .filter-card {
    min-height: 8.8rem;
  }

  .filter-grid-top {
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1.3fr) minmax(0, 1fr);
  }

  .filter-grid-bottom {
    grid-template-columns: minmax(0, 1.05fr) minmax(0, 1.45fr) minmax(280px, 0.78fr);
  }
}
</style>
