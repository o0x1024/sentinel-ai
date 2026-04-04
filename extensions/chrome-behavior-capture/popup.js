const enabledInput = document.getElementById('enabled')
const bridgeUrlInput = document.getElementById('bridgeUrl')
const statusNode = document.getElementById('status')
const lastSeenNode = document.getElementById('lastSeen')
const hintNode = document.getElementById('hint')
const saveButton = document.getElementById('save')
const pingButton = document.getElementById('ping')

function setStatus(ok, text) {
  statusNode.textContent = text
  statusNode.className = `status ${ok ? 'ok' : 'warn'}`
}

function setLastSeen(value) {
  lastSeenNode.textContent = `最近连接时间：${value || '-'}`
}

async function loadSettings() {
  const response = await chrome.runtime.sendMessage({ type: 'sentinel:get-settings' }).catch(error => ({
    ok: false,
    error: String(error),
  }))
  if (!response?.ok) {
    setStatus(false, '扩展后台未就绪')
    hintNode.textContent = response?.error || '扩展后台未响应，请重新加载扩展后再试。'
    return
  }
  const settings = response.settings
  enabledInput.checked = settings.enabled
  bridgeUrlInput.value = settings.bridgeUrl
  setStatus(settings.lastHealthOk, settings.lastHealthOk ? 'Bridge 已连接' : 'Bridge 未连接')
  setLastSeen(settings.lastHealthAt ? new Date(settings.lastHealthAt).toLocaleString() : '-')
  hintNode.textContent = settings.lastError
    ? `最近错误：${settings.lastError}`
    : '先确保 Sentinel 应用已启动，再在代理配置中将行为来源切换为“浏览器扩展行为采集”。'
}

async function saveSettings() {
  const response = await chrome.runtime.sendMessage({
    type: 'sentinel:update-settings',
    patch: {
      enabled: enabledInput.checked,
      bridgeUrl: bridgeUrlInput.value.trim() || 'http://127.0.0.1:18931',
    },
  }).catch(error => ({ ok: false, error: String(error) }))
  if (response?.ok) {
    await loadSettings()
    return
  }
  setStatus(false, '保存失败')
  hintNode.textContent = response?.error || '保存配置失败，请重新加载扩展后再试。'
}

async function pingBridge() {
  const response = await chrome.runtime.sendMessage({ type: 'sentinel:ping-bridge' }).catch(error => ({
    ok: false,
    error: String(error),
  }))
  if (response?.ok) {
    await loadSettings()
    return
  }
  setStatus(false, '连接失败')
  hintNode.textContent = response?.error || 'Bridge 测试连接失败，请确认 Sentinel 已启动且扩展后台正常。'
}

saveButton.addEventListener('click', () => {
  void saveSettings()
})

pingButton.addEventListener('click', () => {
  void pingBridge()
})

void loadSettings()
