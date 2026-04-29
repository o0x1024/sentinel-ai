<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ title }}
    </div>

    <div class="space-y-3 p-4">
      <p class="text-sm text-base-content/70">{{ description }}</p>

      <div class="grid gap-3 lg:grid-cols-[8rem_minmax(0,1fr)]">
        <div class="space-y-2">
          <button class="btn btn-sm w-full" type="button" @click="addBinding">
            {{ $t('trafficAnalysis.intruder.actions.add') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedBinding" @click="configureSelected">
            {{ $t('trafficAnalysis.intruder.actions.configurePlugin') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedBinding" @click="removeSelected">
            {{ $t('trafficAnalysis.intruder.actions.remove') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="selectedIndex <= 0" @click="moveSelected('up')">
            {{ $t('trafficAnalysis.intruder.actions.moveUp') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="selectedIndex < 0 || selectedIndex >= processors.length - 1" @click="moveSelected('down')">
            {{ $t('trafficAnalysis.intruder.actions.moveDown') }}
          </button>
        </div>

        <div class="overflow-hidden rounded-lg border border-base-300">
          <table class="table table-sm">
            <thead class="bg-base-200">
              <tr>
                <th class="w-12"></th>
                <th class="w-40">{{ $t('trafficAnalysis.intruder.labels.extensionPlugin') }}</th>
                <th>{{ $t('trafficAnalysis.intruder.labels.configuration') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="binding in processors"
                :key="binding.id"
                class="cursor-pointer"
                :class="selectedBindingId === binding.id ? 'bg-primary/10' : ''"
                @click="selectedBindingId = binding.id"
              >
                <td>
                  <input
                    :checked="binding.enabled"
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    @click.stop
                    @change="updateBinding(binding.id, { enabled: ($event.target as HTMLInputElement).checked })"
                  />
                </td>
                <td>
                  <select
                    :value="binding.pluginId"
                    class="select select-bordered select-xs w-full"
                    @click.stop
                    @change="handleBindingPluginChange(binding.id, ($event.target as HTMLSelectElement).value)"
                  >
                    <option value="">{{ $t('trafficAnalysis.intruder.labels.pleaseSelect') }}</option>
                    <option v-for="plugin in availablePlugins" :key="plugin.id" :value="plugin.id">{{ plugin.name }}</option>
                  </select>
                </td>
                <td class="max-w-0 truncate text-sm" :title="describeBinding(binding)">
                  {{ describeBinding(binding) }}
                </td>
              </tr>
              <tr v-if="!processors.length">
                <td colspan="3" class="py-10 text-center text-sm text-base-content/60">
                  {{ emptyText }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>

  <IntruderPluginConfigDialog
    :open="configDialogOpen"
    :title="dialogTitle"
    :plugin-id="selectedBinding?.pluginId || ''"
    :preset-name="selectedBinding?.presetName || ''"
    :schema="selectedSchema"
    :model-value="selectedBinding?.config || '{}'"
    @update:model-value="handleConfigUpdate"
    @update:preset-name="handlePresetNameUpdate"
    @close="configDialogOpen = false"
  />
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import { buildResolvedPluginDefaultConfig } from '@/services/pluginDefaultConfig'
import { createIntruderId } from './http'
import IntruderPluginConfigDialog from './IntruderPluginConfigDialog.vue'
import { getIntruderPluginInputSchema, listIntruderPlugins, type IntruderPluginSummary } from './plugins'
import type { IntruderPluginProcessorBinding, IntruderPluginProcessorCategory } from './types'

const props = defineProps<{
  title: string
  description: string
  category: IntruderPluginProcessorCategory
  processors: IntruderPluginProcessorBinding[]
  emptyText: string
}>()

const { t } = useI18n()

const emit = defineEmits<{
  (e: 'update:processors', value: IntruderPluginProcessorBinding[]): void
}>()

const availablePlugins = ref<IntruderPluginSummary[]>([])
const selectedBindingId = ref('')
const configDialogOpen = ref(false)
const schemaCache = ref<Record<string, Record<string, any> | null>>({})
let pluginChangedUnlisten: UnlistenFn | null = null

const selectedIndex = computed(() => props.processors.findIndex((binding) => binding.id === selectedBindingId.value))
const selectedBinding = computed(() => props.processors[selectedIndex.value] ?? null)
const dialogTitle = computed(() => {
  const plugin = availablePlugins.value.find((item) => item.id === selectedBinding.value?.pluginId)
  return plugin?.name || props.title
})
const selectedSchema = computed(() => {
  const pluginId = selectedBinding.value?.pluginId
  return pluginId ? schemaCache.value[pluginId] ?? null : null
})

async function refreshAvailablePlugins() {
  try {
    availablePlugins.value = await listIntruderPlugins(props.category)
  } catch (error) {
    console.error(`Failed to load Intruder plugins for ${props.category}`, error)
  }
}

onMounted(async () => {
  await refreshAvailablePlugins()
  pluginChangedUnlisten = await listen('plugin:changed', async () => {
    await refreshAvailablePlugins()
  })
})

onUnmounted(() => {
  if (pluginChangedUnlisten) {
    pluginChangedUnlisten()
    pluginChangedUnlisten = null
  }
})

watch(
  () => props.processors,
  (processors) => {
    if (!processors.some((binding) => binding.id === selectedBindingId.value)) {
      selectedBindingId.value = processors[0]?.id ?? ''
    }
  },
  { deep: true, immediate: true },
)

function emitProcessors(next: IntruderPluginProcessorBinding[]) {
  emit('update:processors', next)
  if (!next.some((binding) => binding.id === selectedBindingId.value)) {
    selectedBindingId.value = next[0]?.id ?? ''
  }
}

function addBinding() {
  const nextBinding: IntruderPluginProcessorBinding = {
    id: createIntruderId('plugin-processor'),
    pluginId: '',
    presetName: '',
    enabled: true,
    config: '{}',
  }
  emitProcessors([...props.processors, nextBinding])
  selectedBindingId.value = nextBinding.id
}

function updateBinding(id: string, patch: Partial<IntruderPluginProcessorBinding>) {
  emitProcessors(props.processors.map((binding) => binding.id === id ? { ...binding, ...patch } : binding))
}

function removeSelected() {
  if (!selectedBinding.value) return
  emitProcessors(props.processors.filter((binding) => binding.id !== selectedBinding.value?.id))
}

function moveSelected(direction: 'up' | 'down') {
  if (!selectedBinding.value) return
  const index = selectedIndex.value
  const target = direction === 'up' ? index - 1 : index + 1
  if (index < 0 || target < 0 || target >= props.processors.length) return

  const next = [...props.processors]
  const [item] = next.splice(index, 1)
  next.splice(target, 0, item)
  emitProcessors(next)
  selectedBindingId.value = item.id
}

function describeBinding(binding: IntruderPluginProcessorBinding): string {
  const plugin = availablePlugins.value.find((item) => item.id === binding.pluginId)
  if (!binding.pluginId) return t('trafficAnalysis.intruder.labels.noPluginSelected')
  if (!plugin) return binding.pluginId
  const configured = binding.config.trim() && binding.config.trim() !== '{}'
  return configured ? `${plugin.name} (${t('trafficAnalysis.intruder.labels.configured')})` : plugin.name
}

async function handleBindingPluginChange(bindingId: string, pluginId: string) {
  let config = '{}'

  if (pluginId) {
    await ensureSchema(pluginId)
    try {
      const resolvedConfig = await buildResolvedPluginDefaultConfig(pluginId, schemaCache.value[pluginId])
      config = JSON.stringify(resolvedConfig, null, 2)
    } catch (error) {
      console.error(`Failed to load default config for plugin ${pluginId}`, error)
    }
  }

  updateBinding(bindingId, {
    pluginId,
    presetName: '',
    config,
  })
}

async function ensureSchema(pluginId: string) {
  if (schemaCache.value[pluginId] !== undefined) return
  try {
    schemaCache.value[pluginId] = await getIntruderPluginInputSchema(pluginId)
  } catch (error) {
    console.error(`Failed to load schema for plugin ${pluginId}`, error)
    schemaCache.value[pluginId] = null
  }
}

async function configureSelected() {
  if (!selectedBinding.value?.pluginId) {
    dialog.toast.warning(t('trafficAnalysis.intruder.messages.selectPluginFirst'))
    return
  }
  await ensureSchema(selectedBinding.value.pluginId)
  configDialogOpen.value = true
}

function handleConfigUpdate(value: string) {
  if (!selectedBinding.value) return
  updateBinding(selectedBinding.value.id, { config: value })
}

function handlePresetNameUpdate(value: string) {
  if (!selectedBinding.value) return
  updateBinding(selectedBinding.value.id, { presetName: value })
}
</script>
