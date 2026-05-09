<template>
  <div
    class="card bg-base-100 p-3"
    :class="depth > 0 ? 'border border-base-300/70 ml-4' : ''"
  >
    <div class="flex items-start gap-2">
      <div class="flex-1 min-w-0 space-y-2">
        <div v-if="introText && depth === 0" class="text-xs text-base-content/60">
          {{ introText }}
        </div>

        <div class="flex flex-wrap items-end gap-2">
          <div class="form-control min-w-[18rem] flex-1">
            <label class="label py-1">
              <span class="label-text-alt">
                {{ depth === 0 ? t('bugBounty.monitor.primaryPlugin') : t('bugBounty.monitor.fallbackPlugin') }}
              </span>
            </label>
            <select
              v-model="plugin.plugin_id"
              class="select select-sm select-bordered"
              @focus="emit('refresh-plugins')"
              @change="handlePluginChanged"
            >
              <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
              <option v-for="p in pluginOptions" :key="p.id" :value="p.id">
                {{ p.name }}
              </option>
            </select>
          </div>

          <button
            type="button"
            class="btn btn-xs btn-ghost mb-1"
            :disabled="!plugin.plugin_id"
            @click="contentExpanded = !contentExpanded"
          >
            <i class="fas mr-1" :class="contentExpanded ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
            {{ contentExpanded ? t('common.collapse') : t('common.expand') }}
          </button>
        </div>

        <div v-if="plugin.plugin_id" v-show="!contentExpanded" class="flex flex-wrap items-center gap-2 text-xs text-base-content/60">
          <span class="badge badge-sm badge-outline">{{ selectedPluginName }}</span>
          <span v-if="selectedPluginMeta?.seed_bindings?.length" class="badge badge-sm badge-ghost">
            seed bindings {{ selectedPluginMeta.seed_bindings.length }}
          </span>
          <span v-if="plugin.fallback_plugins.length > 0">
            {{ t('bugBounty.monitor.fallbackPlugins') }}: {{ plugin.fallback_plugins.length }}
          </span>
          <span v-if="hasCustomizedParams" class="text-primary">
            {{ t('bugBounty.monitor.customParams') }}
          </span>
        </div>

        <div v-show="contentExpanded" class="space-y-2">
        <MonitorPluginSeedConfigSection
          v-if="selectedPluginMeta?.seed_bindings?.length"
          :plugin="plugin"
          :program-id="programId"
          :declared-bindings="selectedPluginMeta.seed_bindings"
        />

        <div v-if="supportsServiceProbeEngine(plugin)" class="form-control">
          <label class="label py-1">
            <span class="label-text-alt">{{ t('bugBounty.monitor.serviceProbeEngine') }}</span>
          </label>
          <select
            :value="getServiceProbeEngine()"
            class="select select-sm select-bordered"
            @change="handleServiceProbeEngineChange"
          >
            <option value="native">
              {{ t('bugBounty.monitor.serviceProbeEngineNative') }}
            </option>
          </select>
          <div class="text-xs text-base-content/60 mt-1">
            {{ t('bugBounty.monitor.serviceProbeEngineHint') }}
          </div>
        </div>

        <MonitorTargetAssetSelector
          v-if="shouldShowTargetAssetSelector"
          v-model="plugin.target_asset_types"
          :allowed-values="getAllowedTargetAssetTypes(monitorType, plugin.plugin_id)"
        />

        <MonitorSubdomainBruteConfig
          v-if="plugin.plugin_id === 'subdomain_brute'"
          :plugin="plugin"
        />

        <MonitorPluginParamsEditor :plugin="plugin" :monitor-type="monitorType" />

        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-2">
          <label class="label py-1">
            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
          </label>
          <MonitorPluginConfigCard
            v-for="(fallback, fallbackIndex) in plugin.fallback_plugins"
            :key="`${depth}-${fallbackIndex}-${fallback.plugin_id}`"
            :plugin="fallback"
            :monitor-type="monitorType"
            :program-id="programId"
            :plugin-options="pluginOptions"
            :depth="depth + 1"
            :can-remove="true"
            @remove="removeFallback(fallbackIndex)"
            @refresh-plugins="emit('refresh-plugins')"
          />
        </div>

        <button type="button" class="btn btn-xs btn-ghost" @click="addFallbackPlugin">
          <i class="fas fa-plus mr-1"></i>
          {{ t('bugBounty.monitor.addFallback') }}
        </button>
        </div>
      </div>

      <button
        v-if="canRemove"
        type="button"
        class="btn btn-xs btn-ghost text-error"
        @click="emit('remove')"
      >
        <i class="fas fa-trash"></i>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MonitorPluginParamsEditor from './MonitorPluginParamsEditor.vue'
import MonitorPluginSeedConfigSection from './MonitorPluginSeedConfigSection.vue'
import MonitorSubdomainBruteConfig from './MonitorSubdomainBruteConfig.vue'
import MonitorTargetAssetSelector from './MonitorTargetAssetSelector.vue'
import type { MonitorSeedBinding } from '@/components/PluginManagement/seedBindingsSupport'
import {
  createEmptyPluginConfig,
  getAllowedTargetAssetTypes,
  getServiceProbeEngine,
  normalizePluginInputMode,
  resetPluginForSelection,
  setServiceProbeEngine,
  supportsServiceProbeEngine,
  type MonitorPluginConfigLike,
} from './monitorPluginConfigSupport'

defineOptions({ name: 'MonitorPluginConfigCard' })

const props = withDefaults(
  defineProps<{
    plugin: MonitorPluginConfigLike
    monitorType: string
    programId: string
    pluginOptions: Array<{
      id: string
      name: string
      description?: string
      input_mode?: string
      seed_bindings?: MonitorSeedBinding[]
    }>
    depth?: number
    canRemove?: boolean
    introText?: string
  }>(),
  {
    depth: 0,
    canRemove: false,
    introText: '',
  }
)

const emit = defineEmits<{
  (e: 'remove'): void
  (e: 'refresh-plugins'): void
}>()

const { t } = useI18n()
const contentExpanded = ref(!props.plugin.plugin_id)

const selectedPluginName = computed(() => {
  const selected = props.pluginOptions.find(plugin => plugin.id === props.plugin.plugin_id)
  return selected?.name || props.plugin.plugin_id
})

const selectedPluginMeta = computed(() =>
  props.pluginOptions.find(plugin => plugin.id === props.plugin.plugin_id)
)

const selectedPluginInputMode = computed(() =>
  normalizePluginInputMode(selectedPluginMeta.value?.input_mode)
)

const shouldShowTargetAssetSelector = computed(() => selectedPluginInputMode.value !== 'seed')

const hasCustomizedParams = computed(() => {
  const params = props.plugin.plugin_params
  if (!params || typeof params !== 'object' || Array.isArray(params)) {
    return false
  }

  return Object.keys(params).length > 0
})

const handlePluginChanged = () => {
  resetPluginForSelection(props.plugin)
  if (selectedPluginInputMode.value === 'seed') {
    props.plugin.target_asset_types = []
  }
  contentExpanded.value = true
}

const handleServiceProbeEngineChange = (event: Event) => {
  const target = event.target as HTMLSelectElement | null
  setServiceProbeEngine(props.plugin, target?.value || 'native')
}

const addFallbackPlugin = () => {
  props.plugin.fallback_plugins.push(createEmptyPluginConfig())
  contentExpanded.value = true
}

const removeFallback = (index: number) => {
  props.plugin.fallback_plugins.splice(index, 1)
}

watch(
  () => props.plugin.plugin_id,
  (nextPluginId, previousPluginId) => {
    if (nextPluginId !== previousPluginId) {
      if (selectedPluginInputMode.value === 'seed') {
        props.plugin.target_asset_types = []
      }
      contentExpanded.value = true
    }
  },
)
</script>
