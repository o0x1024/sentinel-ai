function getAdminValidationErrors(existingAdmin) {
  const adminId = adminForm.adminId.value.trim()
  const apiKey = adminForm.apiKey.value.trim()
  const errors = {}

  if (!adminId) {
    errors.adminId = 'admin_id 不能为空'
  }
  if (!existingAdmin && !apiKey) {
    errors.apiKey = '创建管理员时必须填写 admin API key'
  }

  return errors
}

function validateAdminForm(existingAdmin) {
  const errors = getAdminValidationErrors(existingAdmin)
  const firstError = Object.values(errors)[0]
  if (firstError) {
    throw new Error(firstError)
  }
}

function syncAdminValidationState() {
  const existingAdmin = state.admins.find(
    item => item.admin_id === adminForm.adminId.value.trim(),
  )
  const errors = getAdminValidationErrors(existingAdmin)
  const showErrors = state.adminFormDirty
  setFieldError(el.adminIdError, showErrors ? errors.adminId || '' : '')
  setFieldError(el.adminApiKeyError, showErrors ? errors.apiKey || '' : '')
  return Object.keys(errors).length === 0
}

function setAdminSaving(saving) {
  state.adminSaving = saving
  el.saveAdminBtn.disabled = saving || !syncAdminValidationState()
  el.saveAdminBtn.textContent = saving ? '保存中...' : '创建 / 更新管理员'
  Object.values(adminForm).forEach(control => {
    control.disabled = saving
  })
}

function syncAdminFormDirtyState() {
  state.adminFormDirty = adminFormSnapshot() !== state.adminFormBaseline
  el.adminDirtyPill.hidden = !state.adminFormDirty
  setAdminSaving(state.adminSaving)
}

async function loadAdmins() {
  const payload = await requestJson('/api/admin/users')
  const users = payload.users || []
  state.admins = users
  el.adminList.innerHTML = users.length
    ? users
        .map(
          user => `
        <article class="list-item">
          <header>
            <strong>${user.admin_id}</strong>
            <span class="pill ${user.active ? 'good' : 'warn'}">${user.role} / ${user.active ? 'active' : 'inactive'}</span>
          </header>
          <div class="muted">Created: ${formatTs(user.created_at)}</div>
          <div class="muted">Updated: ${formatTs(user.updated_at)}</div>
          <div class="muted">Last Used: ${formatTs(user.last_used_at)}</div>
          <div class="toolbar list-toolbar">
            <button class="secondary" data-action="edit-admin" data-admin-id="${user.admin_id}" data-admin-role="${user.role}" data-admin-active="${user.active}">编辑</button>
          </div>
        </article>
      `,
        )
        .join('')
    : '<div class="list-item muted">暂无管理员账号。</div>'
  setAdminSaving(state.adminSaving)
}

function fillAdminForm(adminId, role, active) {
  adminForm.adminId.value = adminId || ''
  adminForm.role.value = role || 'viewer'
  adminForm.active.value = String(active ?? true)
  adminForm.apiKey.value = ''
  markAdminFormClean()
}

async function maybeFillAdminForm(adminId, role, active) {
  if (state.adminFormDirty && !(await confirmDiscardChanges('管理员'))) {
    return
  }
  fillAdminForm(adminId, role, active)
}

async function saveAdmin() {
  const adminId = adminForm.adminId.value.trim()
  const existingAdmin = state.admins.find(item => item.admin_id === adminId)
  validateAdminForm(existingAdmin)

  const role = adminForm.role.value
  const active = adminForm.active.value === 'true'
  const originalRole = existingAdmin?.role || ''
  const originalActive = String(existingAdmin?.active ?? '')
  const body = { role, active }

  if (adminForm.apiKey.value.trim()) {
    body.api_key = adminForm.apiKey.value.trim()
  }

  const deactivationMessage =
    originalActive === 'true'
      ? `管理员 ${adminId} 停用后将无法继续访问 /admin，确认继续吗？`
      : `确认停用管理员 ${adminId} 吗？`

  if (
    !active &&
    !(await confirmDangerAction({
      eyebrow: 'Admin Access',
      title: `停用管理员 ${adminId}`,
      body: deactivationMessage,
      confirmLabel: '确认停用',
      tone: 'danger',
    }))
  ) {
    return
  }

  if (
    originalRole === 'admin' &&
    role !== 'admin' &&
    !(await confirmDangerAction({
      eyebrow: 'Role Change',
      title: `降级管理员 ${adminId}`,
      body: `确认将管理员 ${adminId} 的角色从 admin 降级为 ${role} 吗？\n这会直接收缩该账号的管理权限。`,
      confirmLabel: '确认降级',
      tone: 'warn',
    }))
  ) {
    return
  }

  setAdminSaving(true)
  try {
    const saved = await requestJson(`/api/admin/users/${encodeURIComponent(adminId)}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    })
    showToast(`管理员 ${adminId} 已保存。`, 'success')
    setStatus(el.adminStatus, `管理员 ${adminId} 已保存。`, 'success')
    await Promise.all([loadAdmins(), loadOverview()])
    fillAdminForm(saved.admin_id, saved.role, saved.active)
  } finally {
    setAdminSaving(false)
  }
}

function bindAdminEventHandlers() {
  el.adminList.addEventListener('click', event => {
    const button = event.target.closest('button')
    if (!button || button.dataset.action !== 'edit-admin') return

    setActiveSection('admins')
    maybeFillAdminForm(
      button.dataset.adminId,
      button.dataset.adminRole,
      button.dataset.adminActive === 'true',
    ).catch(err => handleActionError(el.adminStatus, err))
  })

  Object.values(adminForm).forEach(control => {
    control.addEventListener('input', syncAdminFormDirtyState)
    control.addEventListener('change', syncAdminFormDirtyState)
  })
}
