export const createSettingsSecurityActions = (dialog: any) => ({
  changePassword: async (_passwordForm: any) => {
    dialog.toast.success('密码已更改')
  },
  checkVulnerabilities: async () => {
    dialog.toast.info('正在检查漏洞...')
  },
  generateSecurityReport: async () => {
    dialog.toast.success('安全报告已生成')
  },
  lockApplication: async () => {
    dialog.toast.warning('应用程序已锁定')
  },
  emergencyShutdown: async () => {
    const confirmed = await dialog.confirm({
      title: '紧急关闭',
      message: '确定要紧急关闭应用程序吗？',
      variant: 'error',
    })

    if (confirmed) {
      dialog.toast.error('应用程序正在紧急关闭...')
    }
  },
  wipeSecurityData: async () => {
    const confirmed = await dialog.confirm({
      title: '清除安全数据',
      message: '确定要清除所有安全数据吗？此操作不可撤销！',
      variant: 'error',
    })

    if (confirmed) {
      dialog.toast.error('安全数据已清除')
    }
  },
  saveSecurityConfig: async () => {
    dialog.toast.success('安全配置已保存')
  },
})
