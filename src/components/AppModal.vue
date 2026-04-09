<template>
  <Teleport to="body">
    <div
      v-if="open"
      :class="mergedRootClass"
      :style="mergedRootStyle"
      @click.self="$emit('close')"
    >
      <div :class="mergedBoxClass">
        <slot />
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, normalizeClass, useAttrs } from 'vue'
import type { StyleValue } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  open: boolean
  boxClass?: string
  topAligned?: boolean
}>(), {
  boxClass: '',
  topAligned: false,
})

defineEmits<{
  (e: 'close'): void
}>()

const attrs = useAttrs()

const mergedRootClass = computed(() => {
  const className = normalizeClass(attrs.class)
  return [
    'modal',
    'modal-open',
    props.topAligned ? 'app-modal--top' : '',
    className,
  ]
})

const mergedRootStyle = computed<StyleValue | undefined>(() => attrs.style as StyleValue | undefined)
const mergedBoxClass = computed(() => ['modal-box', props.boxClass])
</script>

<style scoped>
.app-modal--top {
  align-items: flex-start;
  padding: 5rem 1rem 1.5rem;
}
</style>
