<template>
  <div class="card bg-base-200 p-3 mb-3">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <div class="flex min-w-0 flex-1 items-center gap-2">
        <button
          type="button"
          class="btn btn-xs btn-ghost h-7 w-7 min-h-0 p-0"
          :title="expanded ? t('common.collapse') : t('common.expand')"
          :disabled="!enabled"
          @click="expanded = !expanded"
        >
          <i class="fas" :class="expanded ? 'fa-chevron-down' : 'fa-chevron-right'"></i>
        </button>
        <label class="label cursor-pointer gap-2 py-0">
          <input
            type="checkbox"
            class="checkbox checkbox-primary"
            :checked="enabled"
            @change="handleEnabledChanged"
          />
          <span class="label-text font-semibold">
            <i :class="iconClass" class="mr-2"></i>
            {{ title }}
          </span>
        </label>
        <span v-if="enabled" class="badge badge-sm badge-outline">
          {{ plugins.length }} {{ t('bugBounty.monitor.pluginConfig') }}
        </span>
      </div>

      <div class="flex items-center gap-2">
        <button
          v-if="enabled"
          type="button"
          class="btn btn-xs btn-ghost"
          @click="emit('add-plugin')"
        >
          <i class="fas fa-plus mr-1"></i>
          {{ t('bugBounty.monitor.addPlugin') }}
        </button>
      </div>
    </div>

    <div v-if="enabled" v-show="expanded" class="mt-3 pl-9">
      <div v-if="plugins.length === 0" class="text-center py-4 text-sm text-base-content/60">
        <i class="fas fa-info-circle mr-1"></i>
        {{ t('bugBounty.monitor.noPluginsConfigured') }}
      </div>

      <div v-else class="space-y-2">
        <MonitorPluginConfigCard
          v-for="(plugin, idx) in plugins"
          :key="`${monitorType}-${idx}`"
          :plugin="plugin"
          :monitor-type="monitorType"
          :plugin-options="pluginOptions"
          :can-remove="true"
          :intro-text="idx === 0 ? introText : ''"
          @remove="emit('remove-plugin', idx)"
          @refresh-plugins="emit('refresh-plugins')"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MonitorPluginConfigCard from './MonitorPluginConfigCard.vue'
import type { MonitorPluginConfigLike } from './monitorPluginConfigSupport'

defineOptions({ name: 'MonitorTypeConfigSection' })

const props = withDefaults(
  defineProps<{
    enabled: boolean
    title: string
    iconClass: string
    monitorType: string
    plugins: MonitorPluginConfigLike[]
    pluginOptions: Array<{ id: string; name: string }>
    introText?: string
  }>(),
  {
    introText: '',
  },
)

const emit = defineEmits<{
  (e: 'update:enabled', value: boolean): void
  (e: 'add-plugin'): void
  (e: 'remove-plugin', index: number): void
  (e: 'refresh-plugins'): void
}>()

const { t } = useI18n()
const expanded = ref(props.enabled)

const handleEnabledChanged = (event: Event) => {
  const checked = Boolean((event.target as HTMLInputElement | null)?.checked)
  emit('update:enabled', checked)
  if (checked) {
    expanded.value = true
  }
}

watch(
  () => props.enabled,
  enabled => {
    if (enabled) {
      expanded.value = true
    }
  },
)
</script>
