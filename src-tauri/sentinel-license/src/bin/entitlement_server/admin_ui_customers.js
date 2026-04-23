function getCustomerValidationErrors() {
  const customerId = customerForm.customerId.value.trim()
  const licenseId = customerForm.licenseId.value.trim()
  const deviceLimit = Number(customerForm.deviceLimit.value)
  const ttlRaw = customerForm.ttlSeconds.value.trim()
  const allowedMachineIds = toLines(customerForm.allowedMachineIds.value)
  const refreshKeyMode = customerForm.refreshKeyMode.value
  const refreshApiKey = customerForm.refreshApiKey.value.trim()
  const errors = {}

  if (!customerId) {
    errors.customerId = 'customer_id 不能为空'
  }
  if (!licenseId) {
    errors.licenseId = 'license_id 不能为空'
  }
  if (!Number.isInteger(deviceLimit) || deviceLimit < 1 || deviceLimit > 64) {
    errors.deviceLimit = 'device_limit 必须是 1 到 64 之间的整数'
  }
  if (ttlRaw) {
    const ttlSeconds = Number(ttlRaw)
    if (!Number.isInteger(ttlSeconds) || ttlSeconds < 300) {
      errors.ttlSeconds = 'ttl_seconds 必须为空，或填写不小于 300 的整数'
    }
  }

  const invalidMachineId = allowedMachineIds.find(value => !isMachineId(value))
  if (invalidMachineId) {
    errors.allowedMachineIds = `allowed_machine_ids 存在无效值：${invalidMachineId}`
  }
  if (refreshKeyMode === 'set' && !refreshApiKey) {
    errors.refreshApiKey = '选择更新 refresh key 时必须输入 refresh_api_key'
  }

  return errors
}

function validateCustomerForm() {
  const errors = getCustomerValidationErrors()
  const firstError = Object.values(errors)[0]
  if (firstError) {
    throw new Error(firstError)
  }
}

function syncCustomerValidationState() {
  const errors = getCustomerValidationErrors()
  const showErrors = state.customerFormDirty
  setFieldError(el.customerIdError, showErrors ? errors.customerId || '' : '')
  setFieldError(el.customerLicenseIdError, showErrors ? errors.licenseId || '' : '')
  setFieldError(el.customerDeviceLimitError, showErrors ? errors.deviceLimit || '' : '')
  setFieldError(el.customerTtlSecondsError, showErrors ? errors.ttlSeconds || '' : '')
  setFieldError(
    el.customerAllowedMachineIdsError,
    showErrors ? errors.allowedMachineIds || '' : '',
  )
  setFieldError(el.customerRefreshApiKeyError, showErrors ? errors.refreshApiKey || '' : '')
  return Object.keys(errors).length === 0
}

function setCustomerSaving(saving) {
  state.customerSaving = saving
  el.saveCustomerBtn.disabled = saving || !syncCustomerValidationState()
  el.resetCustomerFormBtn.disabled = saving
  el.saveCustomerBtn.textContent = saving ? '保存中...' : '创建 / 更新客户'
  Object.values(customerForm).forEach(control => {
    control.disabled = saving
  })
}

function syncCustomerFormDirtyState() {
  state.customerFormDirty = customerFormSnapshot() !== state.customerFormBaseline
  el.customerDirtyPill.hidden = !state.customerFormDirty
  setCustomerSaving(state.customerSaving)
}

async function loadCustomers() {
  const payload = await requestJson('/api/admin/customers')
  state.customers = payload.customers || []
  renderCustomersTable()
}

function renderCustomersTable() {
  const keyword = el.customerSearchInput.value.trim().toLowerCase()
  const customers = state.customers.filter(customer => {
    if (!keyword) return true
    return [customer.customer_id, customer.license_id, customer.tier].some(value =>
      String(value || '').toLowerCase().includes(keyword),
    )
  })

  el.customersTableBody.innerHTML = customers
    .map(
      customer => `
        <tr>
          <td>
            <strong>${customer.customer_id}</strong><br />
            ${customerBadge(customer)}
          </td>
          <td class="mono">${customer.license_id}</td>
          <td>${customer.tier}</td>
          <td>${customer.device_limit}</td>
          <td>
            ${customer.refresh_api_key_configured ? '<span class="pill good">Configured</span>' : '<span class="pill">Global Fallback</span>'}
          </td>
          <td>
            <div class="muted">Key used: ${formatTs(customer.refresh_api_key_last_used_at)}</div>
            <div class="muted">Issued: ${formatTs(customer.last_entitlement_issued_at)}</div>
          </td>
          <td>
            <div class="table-actions">
              <button class="secondary" data-action="edit-customer" data-customer-id="${customer.customer_id}">编辑</button>
              <button class="ghost" data-action="view-devices" data-customer-id="${customer.customer_id}">设备</button>
              <button class="ghost" data-action="filter-audit" data-customer-id="${customer.customer_id}">审计</button>
            </div>
          </td>
        </tr>
      `,
    )
    .join('')
}

function fillCustomerForm(customerId) {
  const customer = state.customers.find(item => item.customer_id === customerId)
  if (!customer) return

  customerForm.customerId.value = customer.customer_id
  customerForm.licenseId.value = customer.license_id
  customerForm.tier.value = customer.tier
  customerForm.deviceLimit.value = customer.device_limit
  customerForm.ttlSeconds.value = customer.ttl_seconds || ''
  customerForm.revoked.value = String(customer.revoked)
  customerForm.featureIds.value = (customer.feature_ids || []).join('\n')
  customerForm.allowedMachineIds.value = (customer.allowed_machine_ids || []).join('\n')
  customerForm.refreshApiKey.value = ''
  customerForm.refreshKeyMode.value = 'preserve'
  markCustomerFormClean()
}

function resetCustomerForm() {
  customerForm.customerId.value = ''
  customerForm.licenseId.value = ''
  customerForm.tier.value = 'pro'
  customerForm.deviceLimit.value = 3
  customerForm.ttlSeconds.value = ''
  customerForm.revoked.value = 'false'
  customerForm.featureIds.value = ''
  customerForm.allowedMachineIds.value = ''
  customerForm.refreshApiKey.value = ''
  customerForm.refreshKeyMode.value = 'preserve'
  markCustomerFormClean()
}

async function maybeFillCustomerForm(customerId) {
  if (state.customerFormDirty && !(await confirmDiscardChanges('客户'))) {
    return
  }
  fillCustomerForm(customerId)
}

async function maybeResetCustomerForm() {
  if (state.customerFormDirty && !(await confirmDiscardChanges('客户'))) {
    return
  }
  resetCustomerForm()
}

async function saveCustomer() {
  validateCustomerForm()

  const customerId = customerForm.customerId.value.trim()
  const refreshKeyMode = customerForm.refreshKeyMode.value
  const refreshApiKey = customerForm.refreshApiKey.value.trim()
  const body = {
    license_id: customerForm.licenseId.value.trim(),
    tier: customerForm.tier.value.trim(),
    feature_ids: toLines(customerForm.featureIds.value),
    device_limit: Number(customerForm.deviceLimit.value || 3),
    revoked: customerForm.revoked.value === 'true',
    allowed_machine_ids: toLines(customerForm.allowedMachineIds.value),
    ttl_seconds: customerForm.ttlSeconds.value ? Number(customerForm.ttlSeconds.value) : null,
    clear_refresh_api_key: refreshKeyMode === 'clear',
  }

  if (refreshKeyMode === 'set') {
    if (!refreshApiKey) {
      throw new Error('选择更新 refresh key 时必须输入 refresh_api_key')
    }
    body.refresh_api_key = refreshApiKey
  }

  if (
    body.revoked &&
    !(await confirmDangerAction({
      eyebrow: 'Customer Risk',
      title: `吊销客户 ${customerId}`,
      body: '这会让该客户后续 entitlement 校验直接失败。\n如果客户端依赖自动刷新 token，相关高价值功能会立即进入受限状态。',
      confirmLabel: '确认吊销',
      tone: 'danger',
    }))
  ) {
    return
  }

  if (
    refreshKeyMode === 'clear' &&
    !(await confirmDangerAction({
      eyebrow: 'Refresh Key',
      title: `清空客户 ${customerId} 的 refresh key`,
      body: '清空后，这个客户将不能再用客户级 refresh key 自动换取 entitlement token。\n如果没有全局 fallback key，自动刷新会直接失败。',
      confirmLabel: '确认清空',
      tone: 'warn',
    }))
  ) {
    return
  }

  setCustomerSaving(true)
  try {
    await requestJson(`/api/admin/customers/${encodeURIComponent(customerId)}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    })
    showToast(`客户 ${customerId} 已保存。`, 'success')
    setStatus(el.customerStatus, `客户 ${customerId} 已保存。`, 'success')
    await Promise.all([loadCustomers(), loadOverview()])
    fillCustomerForm(customerId)
    if (state.selectedCustomerId === customerId) {
      await loadDevices()
    }
  } finally {
    setCustomerSaving(false)
  }
}

function bindCustomerEventHandlers() {
  el.customerSearchInput.addEventListener('input', renderCustomersTable)

  el.customersTableBody.addEventListener('click', event => {
    const button = event.target.closest('button')
    if (!button) return

    const action = button.dataset.action
    const customerId = button.dataset.customerId
    if (!action || !customerId) return

    if (action === 'edit-customer') {
      setActiveSection('customers')
      maybeFillCustomerForm(customerId).catch(err => handleActionError(el.customerStatus, err))
      return
    }

    if (action === 'view-devices') {
      setActiveSection('devices')
      state.selectedCustomerId = customerId
      loadDevices().catch(err => handleActionError(el.authStatus, err))
      return
    }

    if (action === 'filter-audit') {
      setActiveSection('audit')
      el.auditCustomerFilter.value = customerId
      state.auditOffset = 0
      loadAudit().catch(err => handleActionError(el.authStatus, err))
    }
  })

  Object.values(customerForm).forEach(control => {
    control.addEventListener('input', syncCustomerFormDirtyState)
    control.addEventListener('change', syncCustomerFormDirtyState)
  })
}
