<template>
  <!-- daisyUI toast container: fixed position at bottom-end -->
  <div class="toast toast-end toast-bottom z-[9999]">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :class="['alert', getAlertClass(toast.type)]"
        @mouseenter="pauseToast(toast.id)"
        @mouseleave="toast.duration && resumeToast(toast.id, toast.duration)"
      >
        <component :is="getIcon(toast.type)" />
        <span>{{ toast.message }}</span>
        <button
          @click="removeToast(toast.id)"
          class="btn btn-ghost btn-xs btn-circle"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, h } from 'vue'

interface Toast {
  id: string
  type: 'success' | 'error' | 'warning' | 'info'
  message: string
  duration?: number
}

const toasts = ref<Toast[]>([])
const timeouts = new Map<string, ReturnType<typeof setTimeout>>()

// daisyUI alert class mapping
const getAlertClass = (type: string) => {
  const classes: Record<string, string> = {
    success: 'alert-success',
    error: 'alert-error',
    warning: 'alert-warning',
    info: 'alert-info'
  }
  return classes[type] || 'alert-info'
}

// Icon components using daisyUI recommended SVG icons
const getIcon = (type: string) => {
  const icons = {
    success: () => h('svg', {
      xmlns: 'http://www.w3.org/2000/svg',
      class: 'h-6 w-6 shrink-0 stroke-current',
      fill: 'none',
      viewBox: '0 0 24 24'
    }, [
      h('path', {
        'stroke-linecap': 'round',
        'stroke-linejoin': 'round',
        'stroke-width': '2',
        d: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
      })
    ]),
    error: () => h('svg', {
      xmlns: 'http://www.w3.org/2000/svg',
      class: 'h-6 w-6 shrink-0 stroke-current',
      fill: 'none',
      viewBox: '0 0 24 24'
    }, [
      h('path', {
        'stroke-linecap': 'round',
        'stroke-linejoin': 'round',
        'stroke-width': '2',
        d: 'M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z'
      })
    ]),
    warning: () => h('svg', {
      xmlns: 'http://www.w3.org/2000/svg',
      class: 'h-6 w-6 shrink-0 stroke-current',
      fill: 'none',
      viewBox: '0 0 24 24'
    }, [
      h('path', {
        'stroke-linecap': 'round',
        'stroke-linejoin': 'round',
        'stroke-width': '2',
        d: 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z'
      })
    ]),
    info: () => h('svg', {
      xmlns: 'http://www.w3.org/2000/svg',
      class: 'h-6 w-6 shrink-0 stroke-current',
      fill: 'none',
      viewBox: '0 0 24 24'
    }, [
      h('path', {
        'stroke-linecap': 'round',
        'stroke-linejoin': 'round',
        'stroke-width': '2',
        d: 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
      })
    ])
  }
  return icons[type as keyof typeof icons] || icons.info
}

const addToast = (toast: Omit<Toast, 'id'>) => {
  const id = Date.now().toString() + Math.random().toString(36).substr(2, 9)
  const newToast: Toast = {
    id,
    duration: 4000,
    ...toast
  }
  
  toasts.value.push(newToast)
  
  if (newToast.duration && newToast.duration > 0) {
    const timeout = setTimeout(() => {
      removeToast(id)
    }, newToast.duration)
    timeouts.set(id, timeout)
  }
}

const pauseToast = (id: string) => {
  const timeout = timeouts.get(id)
  if (timeout) {
    clearTimeout(timeout)
    timeouts.delete(id)
  }
}

const resumeToast = (id: string, duration: number) => {
  const timeout = setTimeout(() => {
    removeToast(id)
  }, duration)
  timeouts.set(id, timeout)
}

const removeToast = (id: string) => {
  const index = toasts.value.findIndex(toast => toast.id === id)
  if (index > -1) {
    toasts.value.splice(index, 1)
    const timeout = timeouts.get(id)
    if (timeout) {
      clearTimeout(timeout)
      timeouts.delete(id)
    }
  }
}

const clearToasts = () => {
  toasts.value = []
  timeouts.forEach(timeout => clearTimeout(timeout))
  timeouts.clear()
}

const handleToastEvent = (event: CustomEvent) => {
  addToast({
    type: event.detail.type || 'info',
    message: event.detail.message,
    duration: event.detail.duration
  })
}

const onShowToast = (e: Event) => handleToastEvent(e as CustomEvent)

onMounted(() => {
  window.addEventListener('show-toast', onShowToast)
})

onUnmounted(() => {
  window.removeEventListener('show-toast', onShowToast)
  clearToasts()
})

defineExpose({
  addToast,
  removeToast,
  clearToasts
})
</script>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(20px);
}

.toast-move {
  transition: transform 0.3s ease;
}
</style>
