<template>
  <div ref="rootRef" class="inline-flex items-center gap-1">
    <span>{{ label }}</span>
    <button
      ref="triggerRef"
      type="button"
      class="btn btn-xs"
      :class="modelValue ? 'btn-primary' : 'btn-ghost'"
      @click="toggleOpen"
    >
      <i class="fas fa-filter"></i>
    </button>
  </div>

  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[90]"
      @mousedown="onBackdropMouseDown"
    >
      <div
        ref="panelRef"
        class="absolute w-72 rounded-xl border border-base-300 bg-base-100 p-4 shadow-2xl"
        :style="panelStyle"
        @mousedown.stop
      >
        <div class="max-h-64 space-y-3 overflow-auto">
          <template v-if="groups.length > 0">
            <div
              v-for="group in groups"
              :key="group.label"
              class="space-y-2"
            >
              <div class="text-xs font-semibold uppercase tracking-wide text-base-content/50">
                {{ group.label }}
              </div>
              <label
                v-for="option in group.options"
                :key="option.value"
                class="flex cursor-pointer items-center gap-3"
              >
                <input
                  v-model="pendingValue"
                  type="radio"
                  class="radio radio-sm"
                  :value="option.value"
                />
                <span class="text-sm">{{ option.label }}</span>
              </label>
            </div>
          </template>
          <template v-else>
            <label
              v-for="option in options"
              :key="option.value"
              class="flex cursor-pointer items-center gap-3"
            >
              <input
                v-model="pendingValue"
                type="radio"
                class="radio radio-sm"
                :value="option.value"
              />
              <span class="text-sm">{{ option.label }}</span>
            </label>
          </template>
        </div>
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="btn btn-sm" @click="reset">
            {{ t('common.reset') }}
          </button>
          <button type="button" class="btn btn-sm btn-primary" @click="apply">
            {{ t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

interface FilterOption {
  value: string
  label: string
}

interface FilterOptionGroup {
  label: string
  options: FilterOption[]
}

const props = defineProps<{
  label: string
  modelValue: string
  options: FilterOption[]
  groups?: FilterOptionGroup[]
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const { t } = useI18n()

const rootRef = ref<HTMLElement | null>(null)
const triggerRef = ref<HTMLElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
const open = ref(false)
const pendingValue = ref(props.modelValue || '')
const panelPosition = ref({ top: 0, left: 0 })
const groups = computed(() => props.groups || [])

watch(
  () => props.modelValue,
  (value) => {
    pendingValue.value = value || ''
  },
)

const panelStyle = computed(() => ({
  top: `${panelPosition.value.top}px`,
  left: `${panelPosition.value.left}px`,
}))

const updatePanelPosition = () => {
  const trigger = triggerRef.value
  const panel = panelRef.value
  if (!trigger) return

  const rect = trigger.getBoundingClientRect()
  const panelWidth = panel?.offsetWidth || 288
  const viewportWidth = window.innerWidth
  const viewportHeight = window.innerHeight
  const left = Math.min(
    Math.max(12, rect.right - panelWidth),
    Math.max(12, viewportWidth - panelWidth - 12),
  )
  const desiredTop = rect.bottom + 8
  const panelHeight = panel?.offsetHeight || 280
  const top =
    desiredTop + panelHeight > viewportHeight - 12
      ? Math.max(12, rect.top - panelHeight - 8)
      : desiredTop

  panelPosition.value = { top, left }
}

const close = () => {
  open.value = false
}

const onDocumentKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    close()
  }
}

const onWindowResize = () => {
  if (!open.value) return
  updatePanelPosition()
}

const toggleOpen = async () => {
  open.value = !open.value
  if (!open.value) return
  await nextTick()
  updatePanelPosition()
}

const onBackdropMouseDown = () => {
  close()
}

const reset = () => {
  pendingValue.value = ''
  emit('update:modelValue', '')
  close()
}

const apply = () => {
  emit('update:modelValue', pendingValue.value || '')
  close()
}

watch(open, (value) => {
  if (typeof window === 'undefined') return
  if (value) {
    window.addEventListener('resize', onWindowResize)
    window.addEventListener('scroll', onWindowResize, true)
    document.addEventListener('keydown', onDocumentKeydown)
    return
  }
  window.removeEventListener('resize', onWindowResize)
  window.removeEventListener('scroll', onWindowResize, true)
  document.removeEventListener('keydown', onDocumentKeydown)
})

onBeforeUnmount(() => {
  if (typeof window === 'undefined') return
  window.removeEventListener('resize', onWindowResize)
  window.removeEventListener('scroll', onWindowResize, true)
  document.removeEventListener('keydown', onDocumentKeydown)
})
</script>
