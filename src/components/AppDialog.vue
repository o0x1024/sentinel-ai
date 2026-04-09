<template>
  <Teleport to="body">
    <dialog
      ref="dialogRef"
      v-bind="forwardedAttrs"
      :class="mergedClass"
      :style="mergedStyle"
      @close="handleClose"
      @cancel="handleCancel"
    >
      <slot />
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, normalizeClass, onMounted, ref, useAttrs, watch } from 'vue'
import type { StyleValue } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  open?: boolean
}>(), {
  open: false,
})

const attrs = useAttrs()
const dialogRef = ref<HTMLDialogElement | null>(null)
const internalOpen = ref(false)

const normalizedClass = computed(() => normalizeClass(attrs.class))
const mergedClass = computed(() => {
  const className = normalizedClass.value
  return className ? ['modal', className] : ['modal']
})
const mergedStyle = computed<StyleValue | undefined>(() => attrs.style as StyleValue | undefined)
const forwardedAttrs = computed(() => {
  const { class: _class, style: _style, open: _open, ...rest } = attrs
  return rest
})
const isClassControlledOpen = computed(() => normalizedClass.value.split(/\s+/).includes('modal-open'))
const shouldBeOpen = computed(() => props.open || isClassControlledOpen.value || internalOpen.value)

async function syncDialogState() {
  await nextTick()
  const dialog = dialogRef.value
  if (!dialog) {
    return
  }

  if (shouldBeOpen.value) {
    if (!dialog.open) {
      dialog.showModal()
    }
    return
  }

  if (dialog.open) {
    dialog.close()
  }
}

function handleClose() {
  internalOpen.value = false
}

function handleCancel() {
  internalOpen.value = false
}

function showModal() {
  internalOpen.value = true
  void syncDialogState()
}

function close() {
  internalOpen.value = false
  dialogRef.value?.close()
}

function querySelector(selector: string) {
  return dialogRef.value?.querySelector(selector) ?? null
}

function contains(node: Node | null) {
  return node ? dialogRef.value?.contains(node) ?? false : false
}

watch(shouldBeOpen, () => {
  void syncDialogState()
}, { immediate: true })

onMounted(() => {
  void syncDialogState()
})

defineExpose({
  showModal,
  close,
  querySelector,
  contains,
  get open() {
    return dialogRef.value?.open ?? shouldBeOpen.value
  },
})
</script>
