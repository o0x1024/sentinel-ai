import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

export function useTrafficPaneCompactMode(threshold: number) {
  const panelRef = ref<HTMLElement | null>(null)
  const isCompact = ref(false)
  let resizeObserver: ResizeObserver | null = null

  function updateCompactState() {
    isCompact.value = (panelRef.value?.clientWidth ?? 0) < threshold
  }

  function observePanel() {
    resizeObserver?.disconnect()
    resizeObserver = null

    if (!panelRef.value) {
      updateCompactState()
      return
    }

    resizeObserver = new ResizeObserver(() => {
      updateCompactState()
    })
    resizeObserver.observe(panelRef.value)
    updateCompactState()
  }

  watch(panelRef, () => {
    void nextTick(() => {
      observePanel()
    })
  })

  onMounted(() => {
    void nextTick(() => {
      observePanel()
    })
  })

  onUnmounted(() => {
    resizeObserver?.disconnect()
    resizeObserver = null
  })

  return {
    panelRef,
    isCompact,
    updateCompactState,
  }
}
