import { getCurrentInstance } from 'vue'
import { onMounted as vueOnMounted, onUnmounted as vueOnUnmounted, onBeforeUnmount as vueOnBeforeUnmount } from 'vue'

/**
 * Safely register onMounted hook only if component instance exists
 */
export function onMounted(hook: () => void, flush?: 'pre' | 'post') {
  if (getCurrentInstance()) {
    vueOnMounted(hook, flush)
  }
}

/**
 * Safely register onUnmounted hook only if component instance exists
 */
export function onUnmounted(hook: () => void) {
  if (getCurrentInstance()) {
    vueOnUnmounted(hook)
  }
}

/**
 * Safely register onBeforeUnmount hook only if component instance exists
 */
export function onBeforeUnmount(hook: () => void) {
  if (getCurrentInstance()) {
    vueOnBeforeUnmount(hook)
  }
}
