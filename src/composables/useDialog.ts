import { createApp, defineComponent, h, ref, App } from 'vue';
import i18n from '@/i18n';

// 对话框选项接口
interface DialogOptions {
  title?: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'primary' | 'secondary' | 'accent' | 'info' | 'success' | 'warning' | 'error';
}

// 创建一个全局对话框服务
class DialogService {
  private modalId = 'global-dialog-modal';
  private modalContainer: HTMLElement | null = null;
  
  constructor() {
    // 在构造函数中创建模态框容器
    this.ensureModalContainer();
  }

  // 确保模态框容器存在
  private ensureModalContainer() {
    if (!this.modalContainer) {
      // 检查是否已存在
      const existingContainer = document.getElementById(this.modalId);
      
      if (!existingContainer) {
        // 创建模态框容器
        this.modalContainer = document.createElement('dialog');
        this.modalContainer.id = this.modalId;
        this.modalContainer.className = 'modal';
        document.body.appendChild(this.modalContainer);
      } else {
        this.modalContainer = existingContainer;
      }
    }
  }

  // 显示提示对话框
  alert(options: DialogOptions | string): Promise<void> {
    if (typeof options === 'string') {
      options = { message: options };
    }

    return new Promise<void>((resolve) => {
      this.showModal({
        ...options,
        type: 'alert',
        confirmText: options.confirmText || i18n.global.t('common.ok'),
        onConfirm: resolve
      });
    });
  }

  // 显示确认对话框
  confirm(options: DialogOptions | string): Promise<boolean> {
    if (typeof options === 'string') {
      options = { message: options };
    }

    return new Promise<boolean>((resolve) => {
      this.showModal({
        ...options,
        type: 'confirm',
        confirmText: options.confirmText || i18n.global.t('common.confirm'),
        cancelText: options.cancelText || i18n.global.t('common.cancel'),
        onConfirm: () => resolve(true),
        onCancel: () => resolve(false)
      });
    });
  }

  // 显示成功提示
  success(message: string, title?: string): Promise<void> {
    return this.alert({
      title: title || i18n.global.t('common.success'),
      message,
      variant: 'success'
    });
  }

  // 显示错误提示
  error(message: string, title?: string): Promise<void> {
    return this.alert({
      title: title || i18n.global.t('common.error'),
      message,
      variant: 'error'
    });
  }

  // 显示信息提示
  info(options: DialogOptions | string): Promise<void> {
    if (typeof options === 'string') {
      options = { message: options };
    }
    
    return this.alert({
      ...options,
      variant: 'info'
    });
  }

  // 显示模态框
  private showModal(options: any) {
    this.ensureModalContainer();
    
    if (!this.modalContainer) return;
    
    // 获取图标
    const icon = this.getIconForVariant(options.variant);
    
    // 创建模态框内容
    const modalContent = `
      <div class="modal-box">
        <div class="flex items-center gap-3 mb-4">
          ${icon ? `<div class="text-${options.variant} text-2xl">${icon}</div>` : ''}
          <h3 class="font-bold text-lg">${options.title || ''}</h3>
        </div>
        <p class="py-4">${options.message}</p>
        <div class="modal-action">
          ${options.type === 'confirm' ? 
            `<button id="dialog-cancel-btn" class="btn">${options.cancelText}</button>` : ''}
          <button id="dialog-confirm-btn" class="btn ${options.variant ? 'btn-' + options.variant : ''}">${options.confirmText}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    `;
    
    // 设置内容
    this.modalContainer.innerHTML = modalContent;
    
    // 显示模态框
    const modal = this.modalContainer as HTMLDialogElement;
    modal.showModal();
    
    // 添加事件监听
    const confirmBtn = document.getElementById('dialog-confirm-btn');
    if (confirmBtn) {
      confirmBtn.addEventListener('click', () => {
        modal.close();
        if (options.onConfirm) options.onConfirm();
      });
    }
    
    const cancelBtn = document.getElementById('dialog-cancel-btn');
    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => {
        modal.close();
        if (options.onCancel) options.onCancel();
      });
    }
    
    // 点击背景关闭（仅对alert类型）
    if (options.type === 'alert') {
      const backdrop = modal.querySelector('.modal-backdrop');
      if (backdrop) {
        backdrop.addEventListener('click', () => {
          if (options.onConfirm) options.onConfirm();
        });
      }
    }
  }
  
  // 根据变体类型获取图标
  private getIconForVariant(variant: string): string {
    switch (variant) {
      case 'success':
        return '<i class="fas fa-check-circle"></i>';
      case 'error':
        return '<i class="fas fa-exclamation-circle"></i>';
      case 'warning':
        return '<i class="fas fa-exclamation-triangle"></i>';
      case 'info':
        return '<i class="fas fa-info-circle"></i>';
      case 'primary':
        return '<i class="fas fa-bell"></i>';
      default:
        return '';
    }
  }
}

// 导出单例
export const dialog = new DialogService();

// Vue插件
export default {
  install(app: App) {
    app.config.globalProperties.$dialog = dialog;
  }
};

// 类型声明
declare module '@vue/runtime-core' {
  interface ComponentCustomProperties {
    $dialog: DialogService;
  }
} 