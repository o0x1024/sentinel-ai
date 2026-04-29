<template>
  <div ref="triggerRef" class="traffic-variant-switch">
    <button
      type="button"
      class="btn btn-ghost btn-xs gap-1"
      :class="buttonClass"
      :data-testid="testId"
      :aria-expanded="isOpen"
      aria-haspopup="menu"
      @click="toggleMenu"
      @keydown.down.prevent="openMenu"
      @keydown.enter.prevent="toggleMenu"
      @keydown.space.prevent="toggleMenu"
    >
      <span v-if="prefixLabel">{{ prefixLabel }}</span>
      <span :class="activeLabelClass">{{ activeLabel }}</span>
      <i class="fas fa-chevron-down text-[10px]"></i>
    </button>

    <Teleport to="body">
      <ul
        v-if="isOpen"
        ref="menuRef"
        class="traffic-variant-switch-menu menu rounded-box border border-base-300 bg-base-100 p-1 shadow-lg"
        :style="menuStyle"
        role="menu"
        @click.stop
        @keydown.esc.stop.prevent="closeMenu"
      >
        <li>
          <a
            href="#"
            role="menuitem"
            :class="{ active: modelValue === 'original' }"
            @click.prevent="selectVariant('original')"
          >
            {{ originalLabel }}
          </a>
        </li>
        <li>
          <a
            href="#"
            role="menuitem"
            :class="{ active: modelValue === 'edited' }"
            @click.prevent="selectVariant('edited')"
          >
            <span class="text-warning">{{ editedLabel }}</span>
          </a>
        </li>
      </ul>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref } from 'vue'

type TrafficVariant = 'original' | 'edited'

const props = withDefaults(defineProps<{
  modelValue: TrafficVariant
  activeLabel: string
  originalLabel: string
  editedLabel: string
  prefixLabel?: string
  testId?: string
  buttonClass?: string
  menuWidthPx?: number
}>(), {
  prefixLabel: '',
  testId: undefined,
  buttonClass: '',
  menuWidthPx: 160,
})

const emit = defineEmits<{
  'update:modelValue': [value: TrafficVariant]
}>()

const triggerRef = ref<HTMLElement | null>(null)
const menuRef = ref<HTMLElement | null>(null)
const isOpen = ref(false)
const menuPosition = ref({ left: 0, top: 0 })
let positionFrameId: number | null = null

const activeLabelClass = computed(() => (
  props.modelValue === 'edited' ? 'text-warning' : ''
))

const menuStyle = computed(() => ({
  left: `${menuPosition.value.left}px`,
  top: `${menuPosition.value.top}px`,
  width: `${props.menuWidthPx}px`,
}))

function updateMenuPosition() {
  if (positionFrameId !== null) {
    cancelAnimationFrame(positionFrameId)
  }

  positionFrameId = requestAnimationFrame(() => {
    const trigger = triggerRef.value
    if (!trigger) return

    const rect = trigger.getBoundingClientRect()
    const viewportPadding = 8
    const maxLeft = window.innerWidth - props.menuWidthPx - viewportPadding
    const left = Math.max(viewportPadding, Math.min(rect.left, maxLeft))
    const top = Math.max(viewportPadding, rect.bottom + 4)

    menuPosition.value = { left, top }
    positionFrameId = null
  })
}

function addFloatingListeners() {
  document.addEventListener('pointerdown', handleDocumentPointerDown, true)
  window.addEventListener('resize', updateMenuPosition)
  window.addEventListener('scroll', updateMenuPosition, true)
}

function removeFloatingListeners() {
  document.removeEventListener('pointerdown', handleDocumentPointerDown, true)
  window.removeEventListener('resize', updateMenuPosition)
  window.removeEventListener('scroll', updateMenuPosition, true)
}

function openMenu() {
  if (isOpen.value) {
    updateMenuPosition()
    return
  }

  isOpen.value = true
  void nextTick(() => {
    updateMenuPosition()
    addFloatingListeners()
  })
}

function closeMenu() {
  if (!isOpen.value) return
  isOpen.value = false
  removeFloatingListeners()
}

function toggleMenu() {
  if (isOpen.value) {
    closeMenu()
    return
  }
  openMenu()
}

function selectVariant(variant: TrafficVariant) {
  emit('update:modelValue', variant)
  closeMenu()
}

function handleDocumentPointerDown(event: PointerEvent) {
  const target = event.target
  if (!(target instanceof Node)) return
  if (triggerRef.value?.contains(target) || menuRef.value?.contains(target)) return
  closeMenu()
}

onUnmounted(() => {
  removeFloatingListeners()
  if (positionFrameId !== null) {
    cancelAnimationFrame(positionFrameId)
  }
})
</script>

<style scoped>
.traffic-variant-switch {
  display: inline-flex;
  min-width: 0;
}

.traffic-variant-switch-menu {
  position: fixed;
  z-index: 1200;
}
</style>
