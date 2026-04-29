(function sentinelBehaviorCapture() {
  const MAX_TEXT_LENGTH = 120
  const INPUT_EVENT_DEBOUNCE_MS = 600
  const SHELL_POLL_INTERVAL_MS = 1200
  const DEFAULT_SETTINGS = {
    enabled: true,
    bridgeUrl: 'http://127.0.0.1:18931',
    shellCaptureEnabled: true,
    lastHealthOk: false,
    lastHealthAt: null,
    lastError: '',
  }

  let currentSettings = { ...DEFAULT_SETTINGS }
  let inputTimer = null
  let shellPollTimer = null
  let shellPollInFlight = false
  let extensionContextAvailable = true
  let extensionContextLogged = false
  const shellSessions = new Map()
  const shellSessionSync = new Map()
  const isTopFrame = window.top === window

  function isExtensionContextInvalidated(error) {
    return String(error || '').includes('Extension context invalidated')
  }

  function markExtensionContextUnavailable(error) {
    if (!extensionContextAvailable) return
    extensionContextAvailable = false
    extensionContextLogged = true
    if (shellPollTimer) {
      window.clearInterval(shellPollTimer)
      shellPollTimer = null
    }
    shellPollInFlight = false
  }

  function trimText(raw) {
    if (!raw) return ''
    const normalized = String(raw).trim().replace(/\s+/g, ' ')
    return normalized.length > MAX_TEXT_LENGTH
      ? normalized.slice(0, MAX_TEXT_LENGTH)
      : normalized
  }

  function buildSelector(element) {
    const parts = []
    let current = element
    let depth = 0
    while (current && current instanceof Element && depth < 3) {
      let part = current.tagName.toLowerCase()
      if (current.id) {
        part += `#${current.id}`
        parts.unshift(part)
        break
      }
      if (current.classList.length > 0) {
        part += `.${Array.from(current.classList).slice(0, 2).join('.')}`
      }
      parts.unshift(part)
      current = current.parentElement
      depth += 1
    }
    return parts.join(' > ')
  }

  function describeElement(element) {
    if (!(element instanceof Element)) {
      return {}
    }
    return {
      text: trimText(
        element.getAttribute('aria-label')
          || element.getAttribute('title')
          || element.innerText
          || element.textContent,
      ),
      role: trimText(
        element.getAttribute('role')
          || element.tagName.toLowerCase(),
      ),
      selector: trimText(buildSelector(element)),
      inputName: trimText(
        element.getAttribute('name')
          || element.getAttribute('id')
          || element.getAttribute('placeholder'),
      ),
    }
  }

  function baseEvent(eventType, extra = {}) {
    return {
      eventType,
      occurredAt: new Date().toISOString(),
      url: window.location.href,
      host: window.location.host,
      title: document.title,
      route: window.location.pathname + window.location.search + window.location.hash,
      ...extra,
    }
  }

  function emitEvents(events) {
    if (!Array.isArray(events) || events.length === 0) return
    if (!extensionContextAvailable) return
    try {
      chrome.runtime.sendMessage({
        type: 'sentinel:capture-events',
        events,
      }).catch(error => {
        if (isExtensionContextInvalidated(error)) {
          markExtensionContextUnavailable(error)
        }
      })
    } catch (error) {
      if (isExtensionContextInvalidated(error)) {
        markExtensionContextUnavailable(error)
        return
      }
      throw error
    }
  }

  async function loadSettings() {
    if (!extensionContextAvailable) return
    try {
      const stored = await chrome.storage.sync.get(DEFAULT_SETTINGS)
      currentSettings = { ...DEFAULT_SETTINGS, ...stored }
    } catch (error) {
      if (isExtensionContextInvalidated(error)) {
        markExtensionContextUnavailable(error)
        return
      }
      throw error
    }
  }

  async function postShellJson(path, body, options = {}) {
    if (!currentSettings.enabled || !currentSettings.shellCaptureEnabled) return null

    const response = await fetch(`${currentSettings.bridgeUrl}${path}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      keepalive: options.keepalive === true,
      body: JSON.stringify(body),
    })
    const data = await response.json().catch(() => ({}))
    if (!response.ok) {
      throw new Error(data?.error || `Shell bridge request failed: ${response.status}`)
    }
    return data
  }

  function syncShellSession(session) {
    const pending = postShellJson('/v1/browser-shell/sessions/upsert', session)
      .catch(() => null)
    shellSessionSync.set(session.id, pending)
    return pending
  }

  function removeShellSession(sessionId, keepalive = false) {
    const normalizedSessionId = String(sessionId || '').trim()
    if (!normalizedSessionId) return Promise.resolve(null)

    shellSessions.delete(normalizedSessionId)
    shellSessionSync.delete(normalizedSessionId)
    return postShellJson(
      '/v1/browser-shell/sessions/remove',
      { id: normalizedSessionId },
      { keepalive },
    ).catch(() => null)
  }

  function dispatchShellWrite(command) {
    window.postMessage(
      {
        source: 'sentinel-content-script',
        type: 'sentinel:browser-shell-write',
        payload: {
          requestId: command.requestId,
          sessionId: command.sessionId,
          inputText: command.inputText,
        },
      },
      '*',
    )
  }

  async function pollShellCommands() {
    if (
      shellPollInFlight
      || !currentSettings.enabled
      || !currentSettings.shellCaptureEnabled
      || shellSessions.size === 0
    ) {
      return
    }

    shellPollInFlight = true
    try {
      const sessionIds = Array.from(shellSessions.keys()).join(',')
      const response = await fetch(
        `${currentSettings.bridgeUrl}/v1/browser-shell/commands/poll?sessionIds=${encodeURIComponent(sessionIds)}`,
      )
      const data = await response.json().catch(() => ({}))
      if (!response.ok || !data?.ok) return

      const commands = Array.isArray(data.commands) ? data.commands : []
      commands.forEach(dispatchShellWrite)
    } catch (_error) {
      // Keep page behavior untouched when bridge polling fails.
    } finally {
      shellPollInFlight = false
    }
  }

  function ensureShellPolling() {
    if (shellPollTimer) return
    shellPollTimer = window.setInterval(() => {
      void pollShellCommands()
    }, SHELL_POLL_INTERVAL_MS)
  }

  function emitPageLoad() {
    emitEvents([baseEvent('page_load')])
  }

  function emitRouteChange() {
    emitEvents([baseEvent('route_change')])
  }

  function wrapHistoryMethod(method) {
    const original = history[method]
    history[method] = function wrappedHistoryMethod(...args) {
      const result = original.apply(this, args)
      emitRouteChange()
      return result
    }
  }

  window.addEventListener('message', event => {
    if (event.source !== window) return
    if (event.data?.source !== 'sentinel-browser-shell') return

    const payload = event.data.payload || {}
    if (event.data.type === 'sentinel:browser-shell-session-upsert') {
      if (payload.connected === false) {
        void removeShellSession(payload.id)
      } else {
        shellSessions.set(payload.id, payload)
        void syncShellSession(payload)
        ensureShellPolling()
      }
      return
    }

    if (event.data.type === 'sentinel:browser-shell-session-remove') {
      void removeShellSession(payload.sessionId)
      return
    }

    if (event.data.type === 'sentinel:browser-shell-frame') {
      const session = shellSessions.get(payload.sessionId)
      const syncPromise = session ? shellSessionSync.get(payload.sessionId) || syncShellSession(session) : null
      void Promise.resolve(syncPromise)
        .then(() => postShellJson('/v1/browser-shell/frames', payload))
        .catch(() => null)
      return
    }

    if (event.data.type === 'sentinel:browser-shell-write-ack') {
      void postShellJson('/v1/browser-shell/commands/ack', payload)
    }
  })

  try {
    chrome.storage.onChanged.addListener((changes, areaName) => {
      if (areaName !== 'sync') return
      const next = { ...currentSettings }
      Object.entries(changes).forEach(([key, value]) => {
        next[key] = value.newValue
      })
      currentSettings = next
    })
  } catch (error) {
    if (isExtensionContextInvalidated(error)) {
      markExtensionContextUnavailable(error)
    } else {
      throw error
    }
  }

  if (isTopFrame) {
    document.addEventListener('click', event => {
      const target = event.target instanceof Element
        ? event.target.closest('button, a, [role="button"], input, textarea, select') || event.target
        : null
      emitEvents([baseEvent('click', describeElement(target))])
    }, true)

    document.addEventListener('submit', event => {
      const target = event.target instanceof Element ? event.target : null
      emitEvents([baseEvent('submit', describeElement(target))])
    }, true)

    document.addEventListener('change', event => {
      const target = event.target instanceof HTMLInputElement
        || event.target instanceof HTMLTextAreaElement
        || event.target instanceof HTMLSelectElement
        ? event.target
        : null
      if (!target) return

      if (inputTimer) {
        clearTimeout(inputTimer)
      }
      inputTimer = setTimeout(() => {
        emitEvents([baseEvent('input_change', describeElement(target))])
      }, INPUT_EVENT_DEBOUNCE_MS)
    }, true)

    wrapHistoryMethod('pushState')
    wrapHistoryMethod('replaceState')
    window.addEventListener('popstate', emitRouteChange)
    window.addEventListener('hashchange', emitRouteChange)
    window.addEventListener('load', emitPageLoad, { once: true })
    window.addEventListener('pagehide', () => {
      const sessionIds = Array.from(shellSessions.keys())
      sessionIds.forEach((sessionId) => {
        void removeShellSession(sessionId, true)
      })
    })
  }

  void loadSettings()

  if (isTopFrame) {
    emitPageLoad()
  }
})()
