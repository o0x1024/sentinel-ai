import { App } from 'vue'
import i18n from '@/i18n'

// Toast选项接口
interface ToastOptions {
  title?: string
  message: string
  type?: 'success' | 'error' | 'warning' | 'info'
  duration?: number
  width?: 'auto' | 'sm' | 'md' | 'lg' | 'xl' | 'full'
  height?: 'auto' | 'sm' | 'md' | 'lg'
}

// Toast服务类
class ToastService {
  // 显示Toast
  private showToast(options: ToastOptions) {
    const event = new CustomEvent('show-toast', {
      detail: {
        type: options.type || 'info',
        title: options.title,
        message: options.message,
        duration: options.duration,
        width: options.width || 'auto',
        height: options.height || 'auto'
      }
    })
    window.dispatchEvent(event)
  }

  // 显示成功Toast
  success(message: string, title?: string, duration: number = 2000) {
    this.showToast({
      type: 'success',
      message,
      duration
    })
  }

  // 显示错误Toast
  error(message: string, title?: string, duration: number = 3000) {
    this.showToast({
      type: 'error',
      message,
      duration
    })
  }

  // 显示警告Toast
  warning(message: string, title?: string, duration: number = 2000) {
    this.showToast({
      type: 'warning',
      message,
      duration
    })
  }

  // 显示信息Toast
  info(message: string, title?: string, duration: number = 2000) {
    this.showToast({
      type: 'info',
      message,
      duration
    })
  }

  // 通用Toast方法
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