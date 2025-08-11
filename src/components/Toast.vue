<template>
  <div class="toast-container fixed top-20 right-4 z-50 space-y-2">
    <transition-group name="toast" tag="div">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :class="[
          'alert shadow-lg',
          getWidthClass(toast.width || 'auto'),
          getHeightClass(toast.height || 'auto'),
          getToastClass(toast.type)
        ]"
        @mouseenter="pauseToast(toast.id)"
        @mouseleave="toast.duration && resumeToast(toast.id, toast.duration)"
      >
        <div class="flex items-center gap-2">
          <component :is="getIcon(toast.type)" class="w-5 h-5" />
          <div class="flex-1">
            <div class="text-sm">{{ toast.message }}</div>
          </div>
          <button
            @click="removeToast(toast.id)"
            class="btn btn-ghost btn-xs btn-circle"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>
      </div>
    </transition-group>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, h } from 'vue'
import { useI18n } from 'vue-i18n'

interface Toast {
  id: string
  type: 'success' | 'error' | 'warning' | 'info'
  title?: string
  message: string
  duration?: number
  width?: 'auto' | 'sm' | 'md' | 'lg' | 'xl' | 'full'
  height?: 'auto' | 'sm' | 'md' | 'lg'
}

const { t } = useI18n()
const toasts = ref<Toast[]>([])
const timeouts = new Map<string, NodeJS.Timeout>()

const getWidthClass = (width: string) => {
   const widths = {
     auto: 'w-auto max-w-2xl',
     sm: 'max-w-sm',
     md: 'max-w-md',
     lg: 'max-w-lg',
     xl: 'max-w-xl',
     full: 'w-full'
   }
   return widths[width as keyof typeof widths] || 'max-w-md'
 }

const getHeightClass = (height: string) => {
  const heights = {
    auto: 'h-auto',
    sm: 'h-12',
    md: 'h-16', 
    lg: 'h-20'
  }
  return heights[height as keyof typeof heights] || 'h-auto'
}

// 获取Toast样式类
const getToastClass = (type: string) => {
  const classes = {
    success: 'alert-success',
    error: 'alert-error',
    warning: 'alert-warning',
    info: 'alert-info'
  }
  return classes[type as keyof typeof classes] || 'alert-info'
}

// 获取图标组件
const getIcon = (type: string) => {
  const icons = {
    success: () => h('svg', {
      class: 'w-5 h-5',
      fill: 'currentColor',
      viewBox: '0 0 20 20'
    }, [
      h('path', {
        fillRule: 'evenodd',
        d: 'M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z',
        clipRule: 'evenodd'
      })
    ]),
    error: () => h('svg', {
      class: 'w-5 h-5',
      fill: 'currentColor',
      viewBox: '0 0 20 20'
    }, [
      h('path', {
        fillRule: 'evenodd',
        d: 'M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z',
        clipRule: 'evenodd'
      })
    ]),
    warning: () => h('svg', {
      class: 'w-5 h-5',
      fill: 'currentColor',
      viewBox: '0 0 20 20'
    }, [
      h('path', {
        fillRule: 'evenodd',
        d: 'M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z',
        clipRule: 'evenodd'
      })
    ]),
    info: () => h('svg', {
      class: 'w-5 h-5',
      fill: 'currentColor',
      viewBox: '0 0 20 20'
    }, [
      h('path', {
        fillRule: 'evenodd',
        d: 'M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z',
        clipRule: 'evenodd'
      })
    ])
  }
  return icons[type as keyof typeof icons] || icons.info
}

// 添加Toast
const addToast = (toast: Omit<Toast, 'id'>) => {
  const id = Date.now().toString() + Math.random().toString(36).substr(2, 9)
  const newToast: Toast = {
    id,
    duration: 4000,
    ...toast
  }
  
  toasts.value.push(newToast)
  
  // 设置自动移除
  if (newToast.duration && newToast.duration > 0) {
    const timeout = setTimeout(() => {
      removeToast(id)
    }, newToast.duration)
    timeouts.set(id, timeout)
  }
}

// 暂停Toast自动移除
const pauseToast = (id: string) => {
  const timeout = timeouts.get(id)
  if (timeout) {
    clearTimeout(timeout)
    timeouts.delete(id)
  }
}

// 恢复Toast自动移除
const resumeToast = (id: string, duration: number) => {
  const timeout = setTimeout(() => {
    removeToast(id)
  }, duration)
  timeouts.set(id, timeout)
}

// 移除Toast
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

// 清除所有Toast
const clearToasts = () => {
  toasts.value = []
  timeouts.forEach(timeout => clearTimeout(timeout))
  timeouts.clear()
}

// 监听全局事件
const handleToastEvent = (event: CustomEvent) => {
  const { width, height, ...toastData } = event.detail
  addToast({
    ...toastData,
    width: width || 'auto',
    height: height || 'auto'
  })
}

onMounted(() => {
  window.addEventListener('show-toast', handleToastEvent as EventListener)
})

onUnmounted(() => {
  window.removeEventListener('show-toast', handleToastEvent as EventListener)
  clearToasts()
})

// 暴露方法给父组件
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
  transform: translateX(100%);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}

.toast-move {
  transition: transform 0.3s ease;
}
</style>