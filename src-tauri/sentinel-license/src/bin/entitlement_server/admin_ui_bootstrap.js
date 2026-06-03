async function bootstrap() {
  try {
    await loadOverview()
    if (state.token) {
      await Promise.all([loadCustomers(), loadAdmins(), loadAudit()])
      setStatus(el.authStatus, '管理员会话有效。', 'success')
    } else {
      setStatus(el.authStatus, '请输入管理员 API key 后点击“验证并刷新”。')
    }
  } catch (error) {
    setStatus(el.authStatus, String(error.message || error), 'error')
  }
}

function bindSessionEventHandlers() {
  document.getElementById('saveTokenBtn').addEventListener('click', () => {
    state.token = el.tokenInput.value.trim()
    localStorage.setItem('sentinel_entitlement_admin_token', state.token)
    setStatus(el.authStatus, '管理员 token 已保存到浏览器本地。')
    showToast('管理员 token 已保存。', 'success')
  })

  document.getElementById('verifyTokenBtn').addEventListener('click', async () => {
    try {
      state.token = el.tokenInput.value.trim()
      localStorage.setItem('sentinel_entitlement_admin_token', state.token)
      await Promise.all([loadCustomers(), loadAdmins(), loadAudit(), loadOverview()])
      setStatus(el.authStatus, '管理员会话有效，数据已刷新。', 'success')
      showToast('管理员会话验证成功。', 'success')
    } catch (error) {
      handleActionError(el.authStatus, error)
    }
  })

  document.getElementById('clearTokenBtn').addEventListener('click', () => {
    state.token = ''
    localStorage.removeItem('sentinel_entitlement_admin_token')
    el.tokenInput.value = ''
    setStatus(el.authStatus, '管理员 token 已清除。')
    showToast('管理员 token 已清除。')
  })
}

function bindToolbarEventHandlers() {
  document
    .getElementById('refreshOverviewBtn')
    .addEventListener('click', () => loadOverview().catch(err => handleActionError(el.authStatus, err)))

  document
    .getElementById('refreshCustomersBtn')
    .addEventListener('click', () => loadCustomers().catch(err => handleActionError(el.customerStatus, err)))

  document.getElementById('refreshAuditBtn').addEventListener('click', () => {
    state.auditOffset = 0
    loadAudit().catch(err => handleActionError(el.authStatus, err))
  })

  document
    .getElementById('refreshDevicesBtn')
    .addEventListener('click', () => loadDevices().catch(err => handleActionError(el.authStatus, err)))

  document
    .getElementById('refreshAdminsBtn')
    .addEventListener('click', () => loadAdmins().catch(err => handleActionError(el.adminStatus, err)))

  document
    .getElementById('saveCustomerBtn')
    .addEventListener('click', () => saveCustomer().catch(err => handleActionError(el.customerStatus, err)))

  document
    .getElementById('resetCustomerFormBtn')
    .addEventListener('click', () => maybeResetCustomerForm().catch(err => handleActionError(el.customerStatus, err)))

  document
    .getElementById('saveAdminBtn')
    .addEventListener('click', () => saveAdmin().catch(err => handleActionError(el.adminStatus, err)))
}

function bindSidebarEventHandlers() {
  el.sectionNavButtons.forEach(button => {
    button.addEventListener('click', () => {
      setActiveSection(button.dataset.sectionNav || 'overview')
    })
  })

  el.sidebarNewCustomerBtn.addEventListener('click', () => {
    setActiveSection('customers')
    maybeResetCustomerForm().catch(err => handleActionError(el.customerStatus, err))
  })

  el.sidebarNewAdminBtn.addEventListener('click', () => {
    setActiveSection('admins')
    maybeFillAdminForm('', 'admin', true).catch(err => handleActionError(el.adminStatus, err))
  })
}

function bindConfirmDialogEvents() {
  el.confirmCancelBtn.addEventListener('click', () => closeConfirmDialog(false))
  el.confirmSubmitBtn.addEventListener('click', () => closeConfirmDialog(true))
  el.confirmOverlay.addEventListener('click', event => {
    if (event.target === el.confirmOverlay) {
      closeConfirmDialog(false)
    }
  })
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && state.confirmResolve) {
      closeConfirmDialog(false)
    }
  })
}

function bindBeforeUnloadGuard() {
  window.addEventListener('beforeunload', event => {
    if (!state.customerFormDirty && !state.adminFormDirty) return
    event.preventDefault()
    event.returnValue = ''
  })
}

function initializeAdminUi() {
  bindSessionEventHandlers()
  bindToolbarEventHandlers()
  bindSidebarEventHandlers()
  bindCustomerEventHandlers()
  bindAuditEventHandlers()
  bindDeviceEventHandlers()
  bindAdminEventHandlers()
  bindConfirmDialogEvents()
  bindBeforeUnloadGuard()

  resetCustomerForm()
  fillAdminForm('', 'admin', true)
  setCustomerSaving(false)
  setAdminSaving(false)
  setActiveSection('overview')
  updateSidebarContext()
  bootstrap()
}

initializeAdminUi()
