async function loadSelectedCustomer() {
  if (!state.selectedCustomerId) {
    state.selectedCustomer = null
    el.customerDetailCard.className = 'detail-card muted'
    el.customerDetailCard.textContent = '从左侧客户列表中选择一个客户后，这里会显示详情。'
    return
  }

  const customer = await requestJson(
    `/api/admin/customers/${encodeURIComponent(state.selectedCustomerId)}`,
  )
  state.selectedCustomer = customer
  el.customerDetailCard.className = 'detail-card'
  el.customerDetailCard.innerHTML = `
    <header class="detail-card-header">
      <div>
        <strong class="detail-card-title">${customer.customer_id}</strong>
        <div class="muted">${customer.license_id}</div>
      </div>
      ${customer.revoked ? '<span class="pill warn">Revoked</span>' : '<span class="pill good">Active</span>'}
    </header>
    <div class="detail-grid">
      <div><strong>Tier</strong>${customer.tier}</div>
      <div><strong>Device Limit</strong>${customer.device_limit}</div>
      <div><strong>Refresh Key</strong>${customer.refresh_api_key_configured ? 'Configured' : 'Global Fallback'}</div>
      <div><strong>TTL Seconds</strong>${customer.ttl_seconds ?? '—'}</div>
      <div><strong>Refresh Key Used</strong>${formatTs(customer.refresh_api_key_last_used_at)}</div>
      <div><strong>Last Issued</strong>${formatTs(customer.last_entitlement_issued_at)}</div>
    </div>
    <div class="muted detail-meta">Features: ${(customer.feature_ids || []).join(', ') || '—'}</div>
    <div class="muted detail-meta-tight">Allowed Machines: ${(customer.allowed_machine_ids || []).length ? customer.allowed_machine_ids.join(', ') : '—'}</div>
    <div class="toolbar detail-actions">
      <button class="secondary" id="detailEditCustomerBtn">回填到表单</button>
      <button class="ghost" id="detailFilterAuditBtn">只看该客户审计</button>
    </div>
  `

  document.getElementById('detailEditCustomerBtn')?.addEventListener('click', () => {
    setActiveSection('customers')
    maybeFillCustomerForm(customer.customer_id).catch(err => handleActionError(el.customerStatus, err))
  })

  document.getElementById('detailFilterAuditBtn')?.addEventListener('click', () => {
    setActiveSection('audit')
    el.auditCustomerFilter.value = customer.customer_id
    state.auditOffset = 0
    loadAudit().catch(err => handleActionError(el.authStatus, err))
  })
}

async function loadDevices() {
  if (!state.selectedCustomerId) {
    el.deviceList.innerHTML = '<div class="list-item muted">先从客户列表里选择一个客户。</div>'
    el.selectedCustomerPill.textContent = '未选择客户'
    updateSidebarContext()
    await loadSelectedCustomer()
    return
  }

  el.selectedCustomerPill.textContent = state.selectedCustomerId
  updateSidebarContext()
  await loadSelectedCustomer()

  const payload = await requestJson(
    `/api/admin/customers/${encodeURIComponent(state.selectedCustomerId)}/devices`,
  )
  const devices = payload.devices || []
  el.deviceList.innerHTML = devices.length
    ? devices
        .map(
          device => `
        <article class="list-item">
          <header>
            <strong class="mono">${device.machine_id}</strong>
            <button class="danger" data-action="delete-device" data-machine-id="${device.machine_id}">解绑</button>
          </header>
          <div class="muted">Bound: ${formatTs(device.bound_at)}</div>
          <div class="muted">Last Seen: ${formatTs(device.last_seen_at)}</div>
        </article>
      `,
        )
        .join('')
    : '<div class="list-item muted">当前客户没有设备绑定。</div>'
}

async function deleteDevice(machineId) {
  if (!state.selectedCustomerId) return

  const confirmed = await confirmDangerAction({
    eyebrow: 'Device Binding',
    title: `解绑设备 ${machineId}`,
    body: `确认从客户 ${state.selectedCustomerId} 下解绑这台设备吗？\n解绑后，这台设备下次刷新 entitlement 时会重新占用设备额度。`,
    confirmLabel: '确认解绑',
    tone: 'warn',
  })
  if (!confirmed) return

  await requestJson(
    `/api/admin/customers/${encodeURIComponent(state.selectedCustomerId)}/devices/${encodeURIComponent(machineId)}`,
    { method: 'DELETE' },
  )
  showToast(`设备 ${machineId} 已解绑。`, 'success')
  await Promise.all([loadDevices(), loadAudit(), loadOverview()])
}

function bindDeviceEventHandlers() {
  el.deviceList.addEventListener('click', event => {
    const button = event.target.closest('button')
    if (!button || button.dataset.action !== 'delete-device') return
    deleteDevice(button.dataset.machineId).catch(err => handleActionError(el.authStatus, err))
  })
}
