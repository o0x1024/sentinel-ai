<template>
  <div v-if="declaredBindings.length > 0" class="rounded-lg border border-base-300/70 bg-base-200/60 p-3 space-y-3">
    <div class="flex items-center justify-between gap-2">
      <div>
        <div class="text-xs font-semibold text-base-content/80">发现种子</div>
        <div class="text-xs text-base-content/60">
          仅会注入这里显式启用的种子。未配置时不会自动注入。
        </div>
      </div>
      <span v-if="loading" class="loading loading-spinner loading-xs"></span>
    </div>

    <div v-if="loadError" class="text-xs text-warning">
      {{ loadError }}
    </div>

    <div class="space-y-3">
      <div
        v-for="binding in editableBindings"
        :key="binding.key"
        class="rounded-md border border-base-300/70 bg-base-100/70 p-3 space-y-2"
      >
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div class="text-xs font-medium text-base-content/80">
            {{ humanizeSeedType(binding.seed_type) }} -> {{ binding.input_key }}
          </div>
          <div class="flex items-center gap-2">
            <span class="badge badge-sm badge-ghost">
              项目种子 {{ availableSeedCount(binding.seed_type) }}
            </span>
            <label class="label cursor-pointer gap-2 py-0">
              <input
                :checked="binding.use_project_seeds"
                type="checkbox"
                class="checkbox checkbox-xs checkbox-primary"
                @change="updateUseProjectSeeds(binding.key, Boolean(($event.target as HTMLInputElement).checked))"
              />
              <span class="text-xs text-base-content/70">使用项目种子</span>
            </label>
          </div>
        </div>

        <div
          v-if="binding.use_project_seeds"
          class="rounded-md border border-base-300/60 bg-base-200/40 p-2 space-y-2"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="text-xs text-base-content/70">项目种子值</div>
            <div class="flex items-center gap-2">
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                :disabled="binding.project_values.length === 0"
                @click="selectAllProjectValues(binding.key, binding.project_values)"
              >
                全选
              </button>
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                :disabled="binding.selected_project_values.length === 0"
                @click="clearProjectValues(binding.key)"
              >
                清空
              </button>
            </div>
          </div>

          <div v-if="binding.project_values.length === 0" class="text-xs text-base-content/50">
            当前项目没有该类型种子
          </div>

          <div v-else class="max-h-36 overflow-auto space-y-1">
            <label
              v-for="value in binding.project_values"
              :key="`${binding.key}-${value}`"
              class="label cursor-pointer justify-start gap-2 py-1"
            >
              <input
                :checked="binding.selected_project_values.includes(value)"
                type="checkbox"
                class="checkbox checkbox-xs checkbox-primary"
                @change="toggleProjectValue(binding.key, value, Boolean(($event.target as HTMLInputElement).checked))"
              />
              <span class="text-xs font-mono break-all">{{ value }}</span>
            </label>
          </div>
        </div>

        <textarea
          :value="binding.manualText"
          rows="3"
          class="textarea textarea-bordered textarea-sm font-mono w-full"
          placeholder="手工补充，每行一个值"
          @input="updateManualValues(binding.key, ($event.target as HTMLTextAreaElement).value)"
        />

        <div class="text-xs text-base-content/60">
          最终注入 {{ binding.input_key }}:
          <span class="font-mono">{{ formatPreviewValues(binding.preview_values) }}</span>
        </div>
      </div>
    </div>

    <div class="rounded-lg border border-base-300/70 bg-base-100/70 p-3 space-y-2">
      <div class="text-xs font-semibold text-base-content/80">最终种子注入预览</div>
      <pre class="text-xs overflow-auto whitespace-pre-wrap break-all">{{ previewJson }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import type { MonitorSeedBinding } from '@/components/PluginManagement/seedBindingsSupport'
import {
  normalizeSeedBindingConfig,
  type MonitorPluginConfigLike,
} from './monitorPluginConfigSupport'

defineOptions({ name: 'MonitorPluginSeedConfigSection' })

type SurfaceSeedRow = {
  id: string
  program_id: string
  seed_type: string
  seed_value: string
  status: string
}

const props = defineProps<{
  plugin: MonitorPluginConfigLike
  programId: string
  declaredBindings: MonitorSeedBinding[]
}>()

const loading = ref(false)
const loadError = ref('')
const projectSeeds = ref<SurfaceSeedRow[]>([])

const humanizeSeedType = (value: string) => String(value || '').trim().replace(/_/g, ' ')

const bindingKeyOf = (seedType: string, inputKey: string) =>
  `${String(seedType || '').trim().toLowerCase()}::${String(inputKey || '').trim()}`

const findStoredBinding = (seedType: string, inputKey: string) =>
  (props.plugin.seed_config?.bindings || []).find(
    item =>
      String(item?.seed_type || '').trim().toLowerCase() === String(seedType || '').trim().toLowerCase()
      && String(item?.input_key || '').trim() === String(inputKey || '').trim()
  )

const ensureStoredBinding = (seedType: string, inputKey: string) => {
  if (!props.plugin.seed_config || !Array.isArray(props.plugin.seed_config.bindings)) {
    props.plugin.seed_config = { bindings: [] }
  }

  const existing = findStoredBinding(seedType, inputKey)
  if (existing) {
    return existing
  }

  const created = normalizeSeedBindingConfig({
    seed_type: seedType,
    input_key: inputKey,
    use_project_seeds: false,
    selected_project_values: [],
    manual_values: [],
  })

  if (!created) {
    throw new Error(`Invalid seed binding: ${seedType} -> ${inputKey}`)
  }

  props.plugin.seed_config.bindings.push(created)
  return created
}

const ensureSeedBindings = () => {
  const existing = Array.isArray(props.plugin.seed_config?.bindings)
    ? props.plugin.seed_config.bindings
    : []

  const merged = props.declaredBindings.map(binding => {
    const matched = existing.find(
      item => item.seed_type === binding.seed_type && item.input_key === binding.input_key
    )
    return normalizeSeedBindingConfig({
      seed_type: binding.seed_type,
      input_key: binding.input_key,
      use_project_seeds: matched?.use_project_seeds ?? false,
      selected_project_values: matched?.selected_project_values ?? [],
      manual_values: matched?.manual_values ?? [],
    })!
  })

  props.plugin.seed_config = { bindings: merged }
}

const loadSeeds = async () => {
  if (!props.programId) {
    projectSeeds.value = []
    return
  }

  loading.value = true
  loadError.value = ''
  try {
    const seeds = await invoke<SurfaceSeedRow[]>('surface_list_seeds', {
      programId: props.programId,
      status: 'active',
    })
    projectSeeds.value = Array.isArray(seeds) ? seeds : []
  } catch (error) {
    loadError.value = error instanceof Error ? error.message : String(error)
    projectSeeds.value = []
  } finally {
    loading.value = false
  }
}

const projectSeedValuesByType = computed(() => {
  const map = new Map<string, string[]>()
  for (const seed of projectSeeds.value) {
    const key = String(seed.seed_type || '').trim().toLowerCase()
    if (!key) continue
    const list = map.get(key) || []
    if (!list.includes(seed.seed_value)) {
      list.push(seed.seed_value)
    }
    map.set(key, list)
  }
  return map
})

const editableBindings = computed(() =>
  props.declaredBindings.map(declaredBinding => {
    const binding =
      findStoredBinding(declaredBinding.seed_type, declaredBinding.input_key)
      || normalizeSeedBindingConfig({
        seed_type: declaredBinding.seed_type,
        input_key: declaredBinding.input_key,
        use_project_seeds: false,
        selected_project_values: [],
        manual_values: [],
      })!
    const projectValues = binding.use_project_seeds
      ? (projectSeedValuesByType.value.get(binding.seed_type) || [])
      : []
    const selectedProjectValues = binding.use_project_seeds
      ? projectValues.filter(value => (binding.selected_project_values || []).includes(value))
      : []
    const previewValues = Array.from(new Set([...selectedProjectValues, ...(binding.manual_values || [])]))
    return {
      ...binding,
      key: bindingKeyOf(binding.seed_type, binding.input_key),
      manualText: (binding.manual_values || []).join('\n'),
      project_values: projectValues,
      preview_values: previewValues,
    }
  })
)

const previewJson = computed(() => {
  const output: Record<string, string[]> = {}
  for (const binding of editableBindings.value) {
    if (binding.preview_values.length === 0) continue
    output[binding.input_key] = binding.preview_values
  }
  return JSON.stringify(output, null, 2)
})

const availableSeedCount = (seedType: string) =>
  (projectSeedValuesByType.value.get(String(seedType || '').trim().toLowerCase()) || []).length

const updateManualValues = (bindingKey: string, text: string) => {
  let binding = (props.plugin.seed_config?.bindings || []).find(
    item => bindingKeyOf(item.seed_type, item.input_key) === bindingKey
  )
  if (!binding) {
    const [seedType, inputKey] = bindingKey.split('::')
    binding = ensureStoredBinding(seedType, inputKey)
  }
  if (!binding) return
  binding.manual_values = Array.from(
    new Set(
      text
        .split('\n')
        .map(item => item.trim())
        .filter(Boolean)
    )
  )
}

const updateUseProjectSeeds = (bindingKey: string, checked: boolean) => {
  let binding = (props.plugin.seed_config?.bindings || []).find(
    item => bindingKeyOf(item.seed_type, item.input_key) === bindingKey
  )
  if (!binding) {
    const [seedType, inputKey] = bindingKey.split('::')
    binding = ensureStoredBinding(seedType, inputKey)
  }
  if (!binding) return
  binding.use_project_seeds = checked
  if (!checked) {
    binding.selected_project_values = []
  }
}

const toggleProjectValue = (bindingKey: string, value: string, checked: boolean) => {
  let binding = (props.plugin.seed_config?.bindings || []).find(
    item => bindingKeyOf(item.seed_type, item.input_key) === bindingKey
  )
  if (!binding) {
    const [seedType, inputKey] = bindingKey.split('::')
    binding = ensureStoredBinding(seedType, inputKey)
  }
  if (!binding) return
  const current = new Set(binding.selected_project_values || [])
  if (checked) {
    current.add(value)
  } else {
    current.delete(value)
  }
  binding.selected_project_values = Array.from(current)
}

const selectAllProjectValues = (bindingKey: string, values: string[]) => {
  let binding = (props.plugin.seed_config?.bindings || []).find(
    item => bindingKeyOf(item.seed_type, item.input_key) === bindingKey
  )
  if (!binding) {
    const [seedType, inputKey] = bindingKey.split('::')
    binding = ensureStoredBinding(seedType, inputKey)
  }
  if (!binding) return
  binding.selected_project_values = Array.from(new Set(values))
}

const clearProjectValues = (bindingKey: string) => {
  let binding = (props.plugin.seed_config?.bindings || []).find(
    item => bindingKeyOf(item.seed_type, item.input_key) === bindingKey
  )
  if (!binding) {
    const [seedType, inputKey] = bindingKey.split('::')
    binding = ensureStoredBinding(seedType, inputKey)
  }
  if (!binding) return
  binding.selected_project_values = []
}

const formatPreviewValues = (values: string[]) => {
  if (values.length === 0) {
    return '[]'
  }
  return `[${values.join(', ')}]`
}

watch(
  () => props.declaredBindings,
  () => {
    ensureSeedBindings()
  },
  { immediate: true, deep: true },
)

watch(
  () => props.programId,
  () => {
    loadSeeds()
  },
  { immediate: true },
)
</script>
