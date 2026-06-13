<template>
  <div v-if="active" class="tooltip tooltip-bottom" :data-tip="tooltipText">
    <button
      type="button"
      class="badge badge-sm gap-1 cursor-pointer"
      :class="codecViewEnabled ? 'badge-warning' : 'badge-ghost'"
      @click="$emit('toggle')"
    >
      <i :class="codecViewEnabled ? 'fas fa-lock-open' : 'fas fa-lock'" class="text-[10px]"></i>
      <span class="text-[10px]">Codec</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  active: boolean
  codecViewEnabled: boolean
  ruleNames?: string[]
}>()
defineEmits<{ toggle: [] }>()

const { t } = useI18n()

const tooltipText = computed(() => {
  if (!props.active) return ''
  const names = props.ruleNames?.join(', ') ?? ''
  return props.codecViewEnabled
    ? t('trafficAnalysis.codec.badge.toggleDecodeWithRules', { names })
    : t('trafficAnalysis.codec.badge.toggleDecode')
})
</script>
