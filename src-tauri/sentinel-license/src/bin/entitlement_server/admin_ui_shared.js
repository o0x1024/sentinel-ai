const state = {
  token: localStorage.getItem('sentinel_entitlement_admin_token') || '',
  activeSection: 'overview',
  selectedCustomerId: '',
  customers: [],
  admins: [],
  selectedCustomer: null,
  auditOffset: 0,
  confirmResolve: null,
  customerFormBaseline: '',
  adminFormBaseline: '',
  customerFormDirty: false,
  adminFormDirty: false,
  customerSaving: false,
  adminSaving: false,
}

const ERROR_CODE_MESSAGES = {
  missing_authorization: '缺少管理员 Authorization 头，请先填写 Bearer token。',
  invalid_authorization_header: 'Authorization 头格式无效，应为 Bearer <token>。',
  admin_auth_failed: '管理员 API key 无效。',
  admin_inactive: '当前管理员账号已停用。',
  admin_permission_denied: '当前管理员角色没有这个操作权限。',
  admin_rate_limited: '管理接口请求过于频繁，请稍后再试。',
  customer_not_found: '客户不存在。',
  customer_not_entitled: '当前客户没有可用授权。',
  customer_revoked: '客户授权已被吊销。',
  machine_not_allowed: '当前设备不在客户允许的机器白名单里。',
  device_limit_exceeded: '客户绑定设备数已到上限，请先解绑旧设备。',
  refresh_auth_failed: 'refresh 凭据无效。',
  refresh_auth_not_configured: '当前客户没有可用的 refresh 凭据。',
  admin_self_deactivate_forbidden: '不能停用当前正在使用的管理员账号。',
  last_active_admin_forbidden: '系统必须至少保留一个活跃管理员账号。',
  last_admin_role_forbidden: '系统必须至少保留一个活跃的 admin 角色。',
  invalid_customer_id: 'customer_id 不能为空。',
  invalid_machine_id: 'machine_id 格式无效，必须是 64 位十六进制字符串。',
}

const SECTION_LABELS = {
  overview: '运行概况',
  customers: '客户与授权',
  audit: '审计日志',
  devices: '客户详情与设备',
  admins: '管理员账号',
}

const el = {
  tokenInput: document.getElementById('tokenInput'),
  authStatus: document.getElementById('authStatus'),
  healthMetrics: document.getElementById('healthMetrics'),
  customersTableBody: document.getElementById('customersTableBody'),
  customerStatus: document.getElementById('customerStatus'),
  auditList: document.getElementById('auditList'),
  auditCustomerFilter: document.getElementById('auditCustomerFilter'),
  auditEventTypeFilter: document.getElementById('auditEventTypeFilter'),
  auditLimitSelect: document.getElementById('auditLimitSelect'),
  auditPagePill: document.getElementById('auditPagePill'),
  deviceList: document.getElementById('deviceList'),
  selectedCustomerPill: document.getElementById('selectedCustomerPill'),
  customerDetailCard: document.getElementById('customerDetailCard'),
  adminList: document.getElementById('adminList'),
  adminStatus: document.getElementById('adminStatus'),
  customerSearchInput: document.getElementById('customerSearchInput'),
  customerDirtyPill: document.getElementById('customerDirtyPill'),
  adminDirtyPill: document.getElementById('adminDirtyPill'),
  saveCustomerBtn: document.getElementById('saveCustomerBtn'),
  resetCustomerFormBtn: document.getElementById('resetCustomerFormBtn'),
  saveAdminBtn: document.getElementById('saveAdminBtn'),
  customerIdError: document.getElementById('customerIdError'),
  customerLicenseIdError: document.getElementById('customerLicenseIdError'),
  customerDeviceLimitError: document.getElementById('customerDeviceLimitError'),
  customerTtlSecondsError: document.getElementById('customerTtlSecondsError'),
  customerAllowedMachineIdsError: document.getElementById('customerAllowedMachineIdsError'),
  customerRefreshApiKeyError: document.getElementById('customerRefreshApiKeyError'),
  adminIdError: document.getElementById('adminIdError'),
  adminApiKeyError: document.getElementById('adminApiKeyError'),
  confirmOverlay: document.getElementById('confirmOverlay'),
  confirmEyebrow: document.getElementById('confirmEyebrow'),
  confirmTitle: document.getElementById('confirmTitle'),
  confirmBody: document.getElementById('confirmBody'),
  confirmCancelBtn: document.getElementById('confirmCancelBtn'),
  confirmSubmitBtn: document.getElementById('confirmSubmitBtn'),
  toastStack: document.getElementById('toastStack'),
  sidebarSectionLabel: document.getElementById('sidebarSectionLabel'),
  sidebarCustomerLabel: document.getElementById('sidebarCustomerLabel'),
  sidebarNewCustomerBtn: document.getElementById('sidebarNewCustomerBtn'),
  sidebarNewAdminBtn: document.getElementById('sidebarNewAdminBtn'),
  sectionNavButtons: Array.from(document.querySelectorAll('[data-section-nav]')),
  sectionPanels: Array.from(document.querySelectorAll('[data-section-panel]')),
}

const customerForm = {
  customerId: document.getElementById('customerId'),
  licenseId: document.getElementById('customerLicenseId'),
  tier: document.getElementById('customerTier'),
  deviceLimit: document.getElementById('customerDeviceLimit'),
  ttlSeconds: document.getElementById('customerTtlSeconds'),
  revoked: document.getElementById('customerRevoked'),
  featureIds: document.getElementById('customerFeatureIds'),
  allowedMachineIds: document.getElementById('customerAllowedMachineIds'),
  refreshApiKey: document.getElementById('customerRefreshApiKey'),
  refreshKeyMode: document.getElementById('customerRefreshKeyMode'),
}

const adminForm = {
  adminId: document.getElementById('adminId'),
  role: document.getElementById('adminRole'),
  active: document.getElementById('adminActive'),
  apiKey: document.getElementById('adminApiKey'),
}

el.tokenInput.value = state.token

function setStatus(target, message, tone = 'neutral') {
  target.textContent = message
  target.className = 'status-box'
  if (tone === 'error') target.classList.add('error')
  if (tone === 'success') target.classList.add('success')
}

function showToast(message, tone = 'neutral') {
  const toast = document.createElement('div')
  toast.className = `toast ${tone}`.trim()
  toast.textContent = message
  el.toastStack.appendChild(toast)
  window.setTimeout(() => {
    toast.remove()
  }, 4200)
}

function handleActionError(target, error) {
  const message = String(error?.message || error)
  setStatus(target, message, 'error')
  showToast(message, 'error')
}

function isMachineId(value) {
  return /^[a-f0-9]{64}$/.test(value)
}

function toLines(value) {
  return value
    .split('\n')
    .map(item => item.trim())
    .filter(Boolean)
}

function formatTs(value) {
  if (!value) return '—'
  return new Date(value * 1000).toLocaleString()
}

function metricCard(label, value) {
  return `
    <div class="metric">
      <div class="label">${label}</div>
      <div class="value">${value ?? '—'}</div>
    </div>
  `
}

function customerBadge(customer) {
  if (customer.revoked) {
    return '<span class="pill warn">Revoked</span>'
  }
  return '<span class="pill good">Active</span>'
}

function authHeaders() {
  const headers = { 'Content-Type': 'application/json' }
  if (state.token) {
    headers.Authorization = `Bearer ${state.token}`
  }
  return headers
}

function updateSidebarContext() {
  el.sidebarSectionLabel.textContent = SECTION_LABELS[state.activeSection] || '运行概况'
  el.sidebarCustomerLabel.textContent = state.selectedCustomerId || '未选择客户'
}

function setActiveSection(section) {
  state.activeSection = section
  el.sectionNavButtons.forEach(button => {
    button.classList.toggle('active', button.dataset.sectionNav === section)
  })
  el.sectionPanels.forEach(panel => {
    panel.hidden = panel.dataset.sectionPanel !== section
  })
  updateSidebarContext()
}

function formatApiError(payload, response) {
  const code = payload?.code || ''
  const fallback =
    (typeof payload === 'string' && payload) ||
    payload?.error ||
    payload?.message ||
    payload?.detail ||
    `${response.status} ${response.statusText}`
  let message = ERROR_CODE_MESSAGES[code] || fallback
  if (payload?.retry_after_secs) {
    message += `\n建议 ${payload.retry_after_secs} 秒后重试。`
  }
  return {
    code,
    message,
    retryAfterSecs: payload?.retry_after_secs || null,
  }
}

async function requestJson(url, options = {}) {
  const response = await fetch(url, {
    ...options,
    headers: {
      ...authHeaders(),
      ...(options.headers || {}),
    },
  })
  const text = await response.text()
  let payload = null
  try {
    payload = text ? JSON.parse(text) : null
  } catch (_) {
    payload = text
  }
  if (!response.ok) {
    const { code, message, retryAfterSecs } = formatApiError(payload, response)
    if (response.status === 401 || response.status === 403) {
      setStatus(el.authStatus, `管理员会话失效或权限不足：${message}`, 'error')
    }
    const error = new Error(message)
    error.code = code
    error.retryAfterSecs = retryAfterSecs
    throw error
  }
  return payload
}

function closeConfirmDialog(result) {
  el.confirmOverlay.classList.remove('open')
  el.confirmOverlay.setAttribute('aria-hidden', 'true')
  const resolver = state.confirmResolve
  state.confirmResolve = null
  if (resolver) resolver(result)
}

function confirmDangerAction({
  eyebrow = 'Danger Zone',
  title = '确认操作',
  body = '请确认是否继续。',
  confirmLabel = '确认继续',
  tone = 'danger',
}) {
  return new Promise(resolve => {
    state.confirmResolve = resolve
    el.confirmEyebrow.textContent = eyebrow
    el.confirmTitle.textContent = title
    el.confirmBody.textContent = body
    el.confirmSubmitBtn.textContent = confirmLabel
    el.confirmSubmitBtn.className = tone === 'warn' ? 'warn' : 'danger'
    el.confirmOverlay.classList.add('open')
    el.confirmOverlay.setAttribute('aria-hidden', 'false')
  })
}

function customerFormSnapshot() {
  return JSON.stringify({
    customerId: customerForm.customerId.value.trim(),
    licenseId: customerForm.licenseId.value.trim(),
    tier: customerForm.tier.value,
    deviceLimit: customerForm.deviceLimit.value,
    ttlSeconds: customerForm.ttlSeconds.value,
    revoked: customerForm.revoked.value,
    featureIds: customerForm.featureIds.value.trim(),
    allowedMachineIds: customerForm.allowedMachineIds.value.trim(),
    refreshApiKey: customerForm.refreshApiKey.value,
    refreshKeyMode: customerForm.refreshKeyMode.value,
  })
}

function adminFormSnapshot() {
  return JSON.stringify({
    adminId: adminForm.adminId.value.trim(),
    role: adminForm.role.value,
    active: adminForm.active.value,
    apiKey: adminForm.apiKey.value,
  })
}

function setFieldError(target, message = '') {
  target.textContent = message
}

function markCustomerFormClean() {
  state.customerFormBaseline = customerFormSnapshot()
  syncCustomerFormDirtyState()
}

function markAdminFormClean() {
  state.adminFormBaseline = adminFormSnapshot()
  syncAdminFormDirtyState()
}

async function confirmDiscardChanges(scope) {
  return confirmDangerAction({
    eyebrow: 'Unsaved Changes',
    title: `放弃${scope}表单修改`,
    body: `当前${scope}表单还有未保存内容。\n如果继续，刚才的修改会被直接覆盖。`,
    confirmLabel: '放弃修改',
    tone: 'warn',
  })
}
