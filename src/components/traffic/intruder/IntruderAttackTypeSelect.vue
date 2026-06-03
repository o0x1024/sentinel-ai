<template>
  <div ref="containerRef" class="relative">
    <button
      type="button"
      :class="triggerClass"
      aria-haspopup="listbox"
      :aria-expanded="isOpen"
      @click="toggleDropdown"
    >
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-semibold">
          {{ selectedOption.label }}
        </div>
      </div>
      <i
        class="fas fa-chevron-down mt-1 text-xs text-base-content/60 transition-transform"
        :class="{ 'rotate-180': isOpen }"
      ></i>
    </button>

    <div
      v-if="isOpen"
      class="absolute left-0 right-0 z-30 mt-2 overflow-hidden rounded-2xl border border-base-300 bg-base-100 shadow-2xl"
      role="listbox"
      @keydown.esc.prevent="closeDropdown"
    >
      <button
        v-for="(option, index) in options"
        :key="option.value"
        type="button"
        class="flex w-full items-start gap-3 px-4 py-3 text-left transition-colors"
        :class="[
          index > 0 ? 'border-t border-base-300/70' : '',
          option.value === modelValue
            ? 'bg-primary/15 text-base-content'
            : 'hover:bg-base-200/70',
        ]"
        :aria-selected="option.value === modelValue"
        role="option"
        @click="selectOption(option.value)"
      >
        <div class="flex w-4 shrink-0 justify-center pt-0.5">
          <i
            class="fas fa-check text-sm"
            :class="option.value === modelValue ? 'text-base-content' : 'invisible'"
          ></i>
        </div>
        <div class="min-w-0 flex-1">
          <div class="text-sm font-semibold">
            {{ option.label }}
          </div>
          <div class="mt-1 text-xs leading-5 text-base-content/70">
            {{ option.description }}
          </div>
        </div>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { IntruderAttackType } from './types'

interface IntruderAttackTypeOption {
  value: IntruderAttackType
  label: string
  description: string
}

const ATTACK_TYPES: IntruderAttackType[] = ['sniper', 'batteringRam', 'pitchfork', 'clusterBomb']

const props = withDefaults(defineProps<{
  modelValue: IntruderAttackType
  variant?: 'toolbar' | 'field'
}>(), {
  variant: 'field',
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: IntruderAttackType): void
  (e: 'change', value: IntruderAttackType): void
}>()

const { t } = useI18n()
const containerRef = ref<HTMLElement | null>(null)
const isOpen = ref(false)

const options = computed<IntruderAttackTypeOption[]>(() => ATTACK_TYPES.map((value) => ({
  value,
  label: t(`trafficAnalysis.intruder.attackTypes.${value}`),
  description: t(`trafficAnalysis.intruder.attackTypeDescriptions.${value}`),
})))

const selectedOption = computed(() =>
  options.value.find((option) => option.value === props.modelValue) ?? options.value[0],
)

const triggerClass = computed(() =>
  props.variant === 'toolbar'
    ? 'flex h-8 min-h-8 w-full items-center gap-3 rounded-lg border border-base-300 bg-base-100 px-3 text-left shadow-sm transition-colors hover:border-primary/50'
    : 'flex min-h-[2.5rem] w-full items-center gap-3 rounded-lg border border-base-300 bg-base-100 px-3 py-2 text-left transition-colors hover:border-primary/50',
)

function toggleDropdown() {
  isOpen.value = !isOpen.value
}

function closeDropdown() {
  isOpen.value = false
}

function selectOption(value: IntruderAttackType) {
  emit('update:modelValue', value)
  emit('change', value)
  closeDropdown()
}

function handleDocumentPointerDown(event: MouseEvent) {
  if (!isOpen.value) return
  if (!(event.target instanceof Node)) return
  if (containerRef.value?.contains(event.target)) return
  closeDropdown()
}

onMounted(() => {
  document.addEventListener('mousedown', handleDocumentPointerDown)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleDocumentPointerDown)
})
</script>
