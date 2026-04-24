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

        <div class="form-control">
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
            :plugin-options="pluginOptions"
            :depth="depth + 1"
            :can-remove="true"
            @remove="removeFallback(fallbackIndex)"
            @refresh-plugins="emit('refresh-plugins')"
          />
        </div>

        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin">
          <i class="fas fa-plus mr-1"></i>
          {{ t('bugBounty.monitor.addFallback') }}
        </button>
      </div>

      <button
        v-if="canRemove"
        class="btn btn-xs btn-ghost text-error"
        @click="emit('remove')"
      >
        <i class="fas fa-trash"></i>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import MonitorPluginParamsEditor from './MonitorPluginParamsEditor.vue'
import MonitorSubdomainBruteConfig from './MonitorSubdomainBruteConfig.vue'
import MonitorTargetAssetSelector from './MonitorTargetAssetSelector.vue'
import {
  createEmptyPluginConfig,
  getAllowedTargetAssetTypes,
  getServiceProbeEngine,
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
    pluginOptions: Array<{ id: string; name: string }>
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

const handlePluginChanged = () => {
  resetPluginForSelection(props.plugin)
}

const handleServiceProbeEngineChange = (event: Event) => {
  const target = event.target as HTMLSelectElement | null
  setServiceProbeEngine(props.plugin, target?.value || 'native')
}

const addFallbackPlugin = () => {
  props.plugin.fallback_plugins.push(createEmptyPluginConfig())
}

const removeFallback = (index: number) => {
  props.plugin.fallback_plugins.splice(index, 1)
}
</script>
