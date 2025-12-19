import { App } from 'vue'

interface ToastOptions {
  message: string
  type?: 'success' | 'error' | 'warning' | 'info'
  duration?: number
}

class ToastService {
  private showToast(options: ToastOptions) {
    const event = new CustomEvent('show-toast', {
      detail: {
        type: options.type || 'info',
        message: options.message,
        duration: options.duration
      }
    })
    window.dispatchEvent(event)
  }

  success(message: string, duration: number = 2000) {
    this.showToast({ type: 'success', message, duration })
  }

  error(message: string, duration: number = 3000) {
    this.showToast({ type: 'error', message, duration })
  }

  warning(message: string, duration: number = 2000) {
    this.showToast({ type: 'warning', message, duration })
  }

  info(message: string, duration: number = 2000) {
    this.showToast({ type: 'info', message, duration })
  }

  show(options: ToastOptions) {
    this.showToast(options)
  }
}

// 导出单例
export const toast = new ToastService()

// Vue插件
export default {
  install(app: App) {
    app.config.globalProperties.$toast = toast
  }
}

// 类型声明
declare module '@vue/runtime-core' {
  interface ComponentCustomProperties {
    $toast: ToastService
  }
}

// 导出useToast组合式函数
export function useToast() {
  return toast
}