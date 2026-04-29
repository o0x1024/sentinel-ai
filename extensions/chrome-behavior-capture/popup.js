const enabledInput = document.getElementById('enabled')
const shellCaptureEnabledInput = document.getElementById('shellCaptureEnabled')
const bridgeUrlInput = document.getElementById('bridgeUrl')
const statusNode = document.getElementById('status')
const lastSeenNode = document.getElementById('lastSeen')
const shellSessionsNode = document.getElementById('shellSessions')
const hintNode = document.getElementById('hint')
const saveButton = document.getElementById('save')
const pingButton = document.getElementById('ping')
const SHELL_SESSION_REFRESH_MS = 2000
let shellSessionRefreshTimer = null
let extensionContextAvailable = true

function isExtensionContextInvalidated(error) {
  return String(error || '').includes('Extension context invalidated')
}

async function sendRuntimeMessage(message) {
  if (!extensionContextAvailable) {
    return {
      ok: false,
      error: '扩展上下文已失效，请在 chrome://extensions 中重新加载该扩展。',
    }
  }
  try {
    return await chrome.runtime.sendMessage(message)
  } catch (error) {
    if (isExtensionContextInvalidated(error)) {
      extensionContextAvailable = false
      stopShellSessionRefresh()
      return {
        ok: false,
        error: '扩展上下文已失效，请在 chrome://extensions 中重新加载该扩展。',
      }
    }
    return {
      ok: false,
      error: String(error),
    }
  }
}

function setStatus(ok, text) {
  statusNode.textContent = text
  statusNode.className = `status ${ok ? 'ok' : 'warn'}`
}

function setLastSeen(value) {
  lastSeenNode.textContent = `最近连接时间：${value || '-'}`
}

async function loadShellSessions(bridgeUrl) {
  try {
    const response = await fetch(`${bridgeUrl}/v1/browser-shell/sessions`)
    const data = await response.json().catch(() => ({}))
    if (!response.ok || !data?.ok) {
      shellSessionsNode.textContent = '第三方 Shell 会话：读取失败'
      return
    }
    const count = Array.isArray(data.sessions) ? data.sessions.length : 0
    shellSessionsNode.textContent = `第三方 Shell 会话：${count}`
  } catch (_error) {
    shellSessionsNode.textContent = '第三方 Shell 会话：Bridge 未连接'
  }
}

async function loadSettings() {
  const response = await sendRuntimeMessage({ type: 'sentinel:get-settings' })
  if (!response?.ok) {
    setStatus(false, '扩展后台未就绪')
    hintNode.textContent = response?.error || '扩展后台未响应，请重新加载扩展后再试。'
    shellSessionsNode.textContent = '第三方 Shell 会话：扩展上下文失效'
    return
  }
  const settings = response.settings
  enabledInput.checked = settings.enabled
  shellCaptureEnabledInput.checked = settings.shellCaptureEnabled !== false
  bridgeUrlInput.value = settings.bridgeUrl
  setStatus(settings.lastHealthOk, settings.lastHealthOk ? 'Bridge 已连接' : 'Bridge 未连接')
  setLastSeen(settings.lastHealthAt ? new Date(settings.lastHealthAt).toLocaleString() : '-')
  hintNode.textContent = settings.lastError
    ? `最近错误：${settings.lastError}`
    : '先确保 Sentinel 应用已启动，再在代理配置中将行为来源切换为“浏览器扩展行为采集”。'
  await loadShellSessions(settings.bridgeUrl)
}

function startShellSessionRefresh() {
  if (shellSessionRefreshTimer) return
  shellSessionRefreshTimer = window.setInterval(() => {
    const bridgeUrl = bridgeUrlInput.value.trim() || 'http://127.0.0.1:18931'
    void loadShellSessions(bridgeUrl)
  }, SHELL_SESSION_REFRESH_MS)
}

function stopShellSessionRefresh() {
  if (!shellSessionRefreshTimer) return
  window.clearInterval(shellSessionRefreshTimer)
  shellSessionRefreshTimer = null
}

async function saveSettings() {
  const response = await sendRuntimeMessage({
    type: 'sentinel:update-settings',
    patch: {
      enabled: enabledInput.checked,
      shellCaptureEnabled: shellCaptureEnabledInput.checked,
      bridgeUrl: bridgeUrlInput.value.trim() || 'http://127.0.0.1:18931',
    },
  })
  if (response?.ok) {
    await loadSettings()
    return
  }
  setStatus(false, '保存失败')
  hintNode.textContent = response?.error || '保存配置失败，请重新加载扩展后再试。'
}

async function pingBridge() {
  const response = await sendRuntimeMessage({ type: 'sentinel:ping-bridge' })
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
startShellSessionRefresh()

window.addEventListener('beforeunload', () => {
  stopShellSessionRefresh()
})
