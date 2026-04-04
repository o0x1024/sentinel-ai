const DEFAULT_BRIDGE_URL = 'http://127.0.0.1:18931'
const DEFAULT_SETTINGS = {
  enabled: true,
  bridgeUrl: DEFAULT_BRIDGE_URL,
  lastHealthOk: false,
  lastHealthAt: null,
  lastError: '',
}

async function getSettings() {
  const stored = await chrome.storage.sync.get(DEFAULT_SETTINGS)
  return { ...DEFAULT_SETTINGS, ...stored }
}

async function setSettings(patch) {
  const current = await getSettings()
  const next = { ...current, ...patch }
  await chrome.storage.sync.set(next)
  return next
}

async function postEvents(events) {
  const settings = await getSettings()
  if (!settings.enabled || !events.length) {
    return { ok: false, skipped: true }
  }

  const response = await fetch(`${settings.bridgeUrl}/v1/events`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      browser: 'chrome',
      extensionVersion: chrome.runtime.getManifest().version,
      events,
    }),
  })

  const data = await response.json().catch(() => ({}))
  if (!response.ok) {
    throw new Error(data?.error || `Bridge request failed: ${response.status}`)
  }

  await setSettings({
    lastHealthOk: true,
    lastHealthAt: new Date().toISOString(),
    lastError: '',
  })

  return data
}

async function pingBridge() {
  const settings = await getSettings()
  try {
    const response = await fetch(`${settings.bridgeUrl}/health`)
    if (!response.ok) {
      throw new Error(`Health request failed: ${response.status}`)
    }
    await setSettings({
      lastHealthOk: true,
      lastHealthAt: new Date().toISOString(),
      lastError: '',
    })
  } catch (error) {
    await setSettings({
      lastHealthOk: false,
      lastError: String(error),
    })
  }
}

chrome.runtime.onInstalled.addListener(() => {
  void setSettings({})
  void pingBridge()
})

chrome.runtime.onStartup.addListener(() => {
  void pingBridge()
})

const alarmsApi = typeof chrome !== 'undefined' && chrome.alarms ? chrome.alarms : null

if (
  alarmsApi
  && typeof alarmsApi.create === 'function'
  && alarmsApi.onAlarm
  && typeof alarmsApi.onAlarm.addListener === 'function'
) {
  alarmsApi.create('sentinel-bridge-heartbeat', {
    periodInMinutes: 0.5,
  })

  alarmsApi.onAlarm.addListener(alarm => {
    if (alarm.name === 'sentinel-bridge-heartbeat') {
      void postEvents([
        {
          eventType: 'heartbeat',
          url: 'http://127.0.0.1/local-bridge',
          host: '127.0.0.1',
          occurredAt: new Date().toISOString(),
        },
      ]).catch(async error => {
        await setSettings({
          lastHealthOk: false,
          lastError: String(error),
        })
      })
    }
  })
} else {
  void setSettings({
    lastHealthOk: false,
    lastError: 'chrome.alarms API is unavailable. Reload the extension after checking permissions.',
  })
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type === 'sentinel:capture-events') {
    void postEvents(message.events || [])
      .then(data => sendResponse({ ok: true, data }))
      .catch(async error => {
        await setSettings({
          lastHealthOk: false,
          lastError: String(error),
        })
        sendResponse({ ok: false, error: String(error) })
      })
    return true
  }

  if (message?.type === 'sentinel:get-settings') {
    void getSettings().then(settings => sendResponse({ ok: true, settings }))
    return true
  }

  if (message?.type === 'sentinel:update-settings') {
    void setSettings(message.patch || {}).then(settings => sendResponse({ ok: true, settings }))
    return true
  }

  if (message?.type === 'sentinel:ping-bridge') {
    void pingBridge().then(async () => {
      const settings = await getSettings()
      sendResponse({ ok: true, settings })
    })
    return true
  }

  return false
})
