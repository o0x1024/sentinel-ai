(function sentinelPageShellCaptureMain() {
  if (window.__SentinelBrowserShellHookInstalled) return
  if (typeof window.WebSocket !== 'function') return
  window.__SentinelBrowserShellHookInstalled = true

  const SHELL_URL_PATTERN = /(terminal|shell|pty|console|exec|session)/i
  const ANSI_PATTERN = /\u001b\[[0-9;?]*[A-Za-z]/
  const PROMPT_PATTERN = /(^|\n)[^\n]{0,120}[$#%>] $/
  const TERMINAL_ROOT_SELECTOR = [
    '.xterm',
    '[class*="xterm"]',
    '[class*="terminal"]',
    '[class*="Terminal"]',
    '[class*="console"]',
    '[class*="Console"]',
    '[data-testid*="terminal"]',
    '[data-testid*="console"]',
    '[aria-label*="terminal" i]',
    '[aria-label*="console" i]',
  ].join(',')
  const decoder = new TextDecoder()
  const encoder = new TextEncoder()
  const MAX_PREVIEW_CHARS = 4096
  const WRITE_SETTLE_MS = 300
  const WRITE_TIMEOUT_MS = 15000
  const ROOT_SCAN_DEBOUNCE_MS = 250
  let sessionCounter = 0

  function nextSessionId() {
    sessionCounter += 1
    return `browser-shell-${Date.now()}-${sessionCounter}`
  }

  function shouldTrackUrl(url) {
    return typeof url === 'string' && SHELL_URL_PATTERN.test(url)
  }

  function decodeTransportPayload(payload) {
    if (typeof payload === 'string') return payload
    if (payload instanceof ArrayBuffer) return decoder.decode(new Uint8Array(payload))
    if (ArrayBuffer.isView(payload)) return decoder.decode(payload)
    return ''
  }

  function payloadToBytes(payload) {
    if (typeof payload === 'string') return encoder.encode(payload)
    if (payload instanceof ArrayBuffer) return new Uint8Array(payload)
    if (ArrayBuffer.isView(payload)) {
      return new Uint8Array(payload.buffer, payload.byteOffset, payload.byteLength)
    }
    return null
  }

  function encodePayloadBase64(payload) {
    const bytes = payloadToBytes(payload)
    if (!bytes || bytes.length === 0) return null

    let binary = ''
    const chunkSize = 0x8000
    for (let offset = 0; offset < bytes.length; offset += chunkSize) {
      const chunk = bytes.subarray(offset, offset + chunkSize)
      binary += String.fromCharCode(...chunk)
    }
    return window.btoa(binary)
  }

  function buildPreview(payload) {
    const raw = decodeTransportPayload(payload)
    const normalized = raw
      .replace(/\r\n/g, '\n')
      .replace(/\r/g, '\n')
      .replace(/\0/g, '')
    if (!normalized) return ''
    return normalized.length > MAX_PREVIEW_CHARS
      ? normalized.slice(0, MAX_PREVIEW_CHARS)
      : normalized
  }

  function looksLikeShellPayload(payload) {
    const text = decodeTransportPayload(payload)
    if (!text) return false
    return ANSI_PATTERN.test(text) || PROMPT_PATTERN.test(text)
  }

  function inferFrameType(payload) {
    return typeof payload === 'string' ? 'text' : 'binary'
  }

  function protocolToString(protocols) {
    if (typeof protocols === 'string') return protocols
    if (Array.isArray(protocols)) return protocols.join(',')
    return ''
  }

  function normalizeWriteInput(inputText) {
    const raw = typeof inputText === 'string' ? inputText : String(inputText || '')
    if (!raw) return raw
    const normalized = raw
      .replace(/\r\n/g, '\n')
      .replace(/\r/g, '\n')
      .replace(/\n/g, '\r')
    if (normalized.endsWith('\r')) return normalized
    return `${normalized}\r`
  }

  function emitToExtension(type, payload) {
    window.postMessage(
      {
        source: 'sentinel-browser-shell',
        type,
        payload,
      },
      '*',
    )
  }

  const sessionsById = new Map()
  const sessionIdsBySocket = new WeakMap()
  const trackedSockets = new Map()
  const suppressedSockets = new WeakSet()
  const activeWritesBySession = new Map()
  const queuedWritesBySession = new Map()
  const sessionRootsById = new Map()
  const terminalInstancesBySession = new Map()
  const sessionIdsByTerminal = new WeakMap()
  let rootLifecycleScanTimerId = null

  function clearRootLifecycleScanTimer() {
    if (rootLifecycleScanTimerId) {
      window.clearTimeout(rootLifecycleScanTimerId)
      rootLifecycleScanTimerId = null
    }
  }

  function scheduleRootLifecycleScan() {
    if (rootLifecycleScanTimerId) return
    rootLifecycleScanTimerId = window.setTimeout(() => {
      rootLifecycleScanTimerId = null
      scanTrackedTerminalRoots()
    }, ROOT_SCAN_DEBOUNCE_MS)
  }

  function scanTrackedTerminalRoots() {
    for (const [sessionId, root] of sessionRootsById.entries()) {
      if (!isTerminalRootInteractive(root)) {
        removeSessionTracking(sessionId, 'terminal_root_unavailable')
      }
    }
  }

  function collectLikelyTerminalRoots() {
    const roots = new Set()
    const activeElement = document.activeElement

    if (activeElement instanceof Element) {
      const closestRoot = activeElement.closest(TERMINAL_ROOT_SELECTOR)
      if (closestRoot) {
        roots.add(closestRoot)
      }
    }

    document.querySelectorAll(TERMINAL_ROOT_SELECTOR).forEach((node) => {
      if (node instanceof Element) {
        roots.add(node)
      }
    })

    return [...roots]
  }

  function terminalRootScore(root) {
    if (!(root instanceof Element)) return Number.NEGATIVE_INFINITY

    let score = 0
    const text = [
      root.className,
      root.id,
      root.getAttribute('data-testid') || '',
      root.getAttribute('aria-label') || '',
    ].join(' ').toLowerCase()

    if (text.includes('xterm')) score += 120
    if (text.includes('terminal')) score += 80
    if (text.includes('console')) score += 60
    if (text.includes('shell')) score += 50
    if (text.includes('pty')) score += 40
    if (root.querySelector('.xterm-screen, .xterm-rows')) score += 80
    if (root.querySelector('canvas, textarea, [contenteditable="true"]')) score += 20
    if (document.activeElement instanceof Element && root.contains(document.activeElement)) {
      score += 40
    }
    if (isTerminalRootInteractive(root)) {
      score += 40
    }

    return score
  }

  function isTerminalRootInteractive(root) {
    if (!(root instanceof Element)) return false
    if (!root.isConnected) return false

    const hiddenAncestor = root.closest('[hidden], [aria-hidden="true"], dialog:not([open])')
    if (hiddenAncestor) return false

    let current = root
    while (current instanceof Element) {
      const style = window.getComputedStyle(current)
      if (style.display === 'none' || style.visibility === 'hidden') {
        return false
      }
      current = current.parentElement
    }

    return true
  }

  function findLikelyTerminalRoot() {
    const roots = collectLikelyTerminalRoots()
    if (roots.length === 0) return null

    let bestRoot = null
    let bestScore = Number.NEGATIVE_INFINITY
    for (const root of roots) {
      const score = terminalRootScore(root)
      if (score > bestScore) {
        bestScore = score
        bestRoot = root
      }
    }
    return bestScore > 0 ? bestRoot : null
  }

  function bindTerminalRootToSession(sessionId, root) {
    if (!sessionId || !(root instanceof Element)) return
    sessionRootsById.set(sessionId, root)
  }

  function bindTerminalInstanceToSession(sessionId, terminalInstance) {
    if (!sessionId || !terminalInstance || typeof terminalInstance !== 'object') return
    terminalInstancesBySession.set(sessionId, terminalInstance)
    sessionIdsByTerminal.set(terminalInstance, sessionId)
  }

  function getSessionByTerminalInstance(terminalInstance) {
    const sessionId = sessionIdsByTerminal.get(terminalInstance)
    return sessionId ? sessionsById.get(sessionId) || null : null
  }

  function selectSessionForTerminalBinding() {
    let selectedSession = null
    for (const session of sessionsById.values()) {
      if (!session.connected) continue
      if (!trackedSockets.has(session.id)) continue
      if (sessionRootsById.has(session.id)) continue
      if (
        !selectedSession
        || session.lastSeenAt > selectedSession.lastSeenAt
      ) {
        selectedSession = session
      }
    }
    return selectedSession
  }

  function maybeBindTerminalRoot(sessionId) {
    if (!sessionId || sessionRootsById.has(sessionId)) return
    const root = findLikelyTerminalRoot()
    if (root) {
      bindTerminalRootToSession(sessionId, root)
    }
  }

  function bindTerminalLifecycle(sessionId, terminalInstance, root) {
    const normalizedSessionId = String(sessionId || '').trim()
    if (!normalizedSessionId) return

    if (root instanceof Element) {
      bindTerminalRootToSession(normalizedSessionId, root)
    }
    bindTerminalInstanceToSession(normalizedSessionId, terminalInstance)
  }

  function bindTerminalLifecycleToBestSession(terminalInstance, root) {
    const existing = getSessionByTerminalInstance(terminalInstance)
    if (existing) {
      bindTerminalLifecycle(existing.id, terminalInstance, root)
      return
    }

    const selectedSession = selectSessionForTerminalBinding()
    if (!selectedSession) return
    bindTerminalLifecycle(selectedSession.id, terminalInstance, root)
  }

  function removeSessionTracking(sessionId, reason) {
    const normalizedSessionId = String(sessionId || '').trim()
    if (!normalizedSessionId) return

    const socket = trackedSockets.get(normalizedSessionId) || null
    if (socket) {
      suppressedSockets.add(socket)
      sessionIdsBySocket.delete(socket)
    }

    trackedSockets.delete(normalizedSessionId)
    queuedWritesBySession.delete(normalizedSessionId)
    sessionRootsById.delete(normalizedSessionId)
    terminalInstancesBySession.delete(normalizedSessionId)
    sessionsById.delete(normalizedSessionId)
    completeActiveWrite(normalizedSessionId, false, `Browser shell session removed: ${reason}`)

    emitToExtension('sentinel:browser-shell-session-remove', {
      sessionId: normalizedSessionId,
      reason,
    })
  }

  function upsertSession(socket, patch) {
    let sessionId = sessionIdsBySocket.get(socket)
    let current = sessionId ? sessionsById.get(sessionId) : null
    if (!current) {
      sessionId = nextSessionId()
      current = {
        id: sessionId,
        tabId: null,
        frameId: null,
        pageUrl: window.location.href,
        pageTitle: document.title || '',
        wsUrl: patch.wsUrl || '',
        protocol: patch.protocol || '',
        terminalKind: patch.terminalKind || 'websocket_shell',
        writable: patch.writable !== false,
        connected: patch.connected !== false,
        lastSeenAt: new Date().toISOString(),
      }
    }

    const next = {
      ...current,
      ...patch,
      id: sessionId,
      pageUrl: patch.pageUrl || window.location.href,
      pageTitle: patch.pageTitle || document.title || current.pageTitle || '',
      lastSeenAt: new Date().toISOString(),
    }
    sessionsById.set(sessionId, next)
    sessionIdsBySocket.set(socket, sessionId)
    trackedSockets.set(sessionId, socket)
    maybeBindTerminalRoot(sessionId)
    emitToExtension('sentinel:browser-shell-session-upsert', next)
    return next
  }

  function getSessionBySocket(socket) {
    const sessionId = sessionIdsBySocket.get(socket)
    return sessionId ? sessionsById.get(sessionId) || null : null
  }

  function getSessionById(sessionId) {
    return sessionsById.get(sessionId) || null
  }

  function emitFrame(socket, direction, payload) {
    const current = getSessionBySocket(socket)
    if (!current) return
    emitToExtension('sentinel:browser-shell-frame', {
      sessionId: current.id,
      direction,
      frameType: inferFrameType(payload),
      textPreview: buildPreview(payload),
      payloadBase64: encodePayloadBase64(payload),
      occurredAt: new Date().toISOString(),
    })
  }

  function clearTimer(timerId) {
    if (timerId) {
      window.clearTimeout(timerId)
    }
  }

  function dequeueNextWrite(sessionId) {
    const queue = queuedWritesBySession.get(sessionId)
    if (!queue || queue.length === 0) {
      queuedWritesBySession.delete(sessionId)
      return null
    }
    const next = queue.shift() || null
    if (queue.length === 0) {
      queuedWritesBySession.delete(sessionId)
    }
    return next
  }

  function completeActiveWrite(sessionId, success, error) {
    const activeWrite = activeWritesBySession.get(sessionId)
    if (!activeWrite) return

    clearTimer(activeWrite.settleTimerId)
    clearTimer(activeWrite.timeoutTimerId)
    activeWritesBySession.delete(sessionId)

    emitToExtension('sentinel:browser-shell-write-ack', {
      requestId: activeWrite.requestId,
      sessionId,
      success,
      error: error || null,
    })

    window.setTimeout(() => {
      void pumpWriteQueue(sessionId)
    }, 0)
  }

  function scheduleWriteSettle(sessionId) {
    const activeWrite = activeWritesBySession.get(sessionId)
    if (!activeWrite) return

    clearTimer(activeWrite.settleTimerId)
    activeWrite.settleTimerId = window.setTimeout(() => {
      completeActiveWrite(sessionId, true, null)
    }, WRITE_SETTLE_MS)
  }

  function trackInboundForActiveWrite(sessionId, payload) {
    const activeWrite = activeWritesBySession.get(sessionId)
    if (!activeWrite) return

    const preview = buildPreview(payload)
    activeWrite.observedInbound = true
    activeWrite.lastInboundPreview = preview
    if (PROMPT_PATTERN.test(preview)) {
      scheduleWriteSettle(sessionId)
      return
    }
    scheduleWriteSettle(sessionId)
  }

  function enqueueWrite(payload) {
    const sessionId = String(payload.sessionId || '').trim()
    if (!sessionId) return
    const queue = queuedWritesBySession.get(sessionId) || []
    queue.push(payload)
    queuedWritesBySession.set(sessionId, queue)
    void pumpWriteQueue(sessionId)
  }

  function failQueuedWrite(payload, error) {
    emitToExtension('sentinel:browser-shell-write-ack', {
      requestId: payload.requestId,
      sessionId: payload.sessionId,
      success: false,
      error,
    })
  }

  function pumpWriteQueue(sessionId) {
    if (activeWritesBySession.has(sessionId)) return

    const nextWrite = dequeueNextWrite(sessionId)
    if (!nextWrite) return

    const session = getSessionById(sessionId)
    if (!session) {
      failQueuedWrite(nextWrite, 'Session not found in page context')
      void pumpWriteQueue(sessionId)
      return
    }

    const socket = trackedSockets.get(sessionId) || null
    if (!socket || socket.readyState !== NativeWebSocket.OPEN) {
      failQueuedWrite(nextWrite, 'Socket not open for session')
      void pumpWriteQueue(sessionId)
      return
    }

    const activeWrite = {
      requestId: nextWrite.requestId,
      sessionId,
      observedInbound: false,
      lastInboundPreview: '',
      settleTimerId: null,
      timeoutTimerId: null,
    }
    activeWritesBySession.set(sessionId, activeWrite)

    activeWrite.timeoutTimerId = window.setTimeout(() => {
      const suffix = activeWrite.observedInbound
        ? `Last inbound preview: ${activeWrite.lastInboundPreview || '[empty]'}`
        : 'No inbound terminal output observed after write dispatch'
      completeActiveWrite(sessionId, false, `Browser shell write timed out. ${suffix}`)
    }, WRITE_TIMEOUT_MS)

    try {
      socket.send(normalizeWriteInput(nextWrite.inputText))
    } catch (error) {
      completeActiveWrite(sessionId, false, String(error))
    }
  }

  const NativeWebSocket = window.WebSocket

  function installTerminalLifecycleHook(constructorName, TerminalConstructor) {
    if (typeof TerminalConstructor !== 'function') return
    const prototype = TerminalConstructor.prototype
    if (!prototype || prototype.__sentinelLifecycleHookInstalled) return

    const nativeOpen = prototype.open
    if (typeof nativeOpen === 'function') {
      prototype.open = function sentinelTerminalOpen(root, ...args) {
        const result = nativeOpen.call(this, root, ...args)
        bindTerminalLifecycleToBestSession(this, root)
        return result
      }
    }

    const nativeDispose = prototype.dispose
    if (typeof nativeDispose === 'function') {
      prototype.dispose = function sentinelTerminalDispose(...args) {
        const session = getSessionByTerminalInstance(this)
        if (session) {
          removeSessionTracking(session.id, `${constructorName}.dispose`)
        }
        return nativeDispose.call(this, ...args)
      }
    }

    prototype.__sentinelLifecycleHookInstalled = true
  }

  function interceptTerminalConstructor(constructorName) {
    let currentValue = window[constructorName]
    if (currentValue) {
      installTerminalLifecycleHook(constructorName, currentValue)
    }

    Object.defineProperty(window, constructorName, {
      configurable: true,
      enumerable: true,
      get() {
        return currentValue
      },
      set(nextValue) {
        currentValue = nextValue
        installTerminalLifecycleHook(constructorName, nextValue)
      },
    })
  }

  class SentinelWrappedWebSocket extends NativeWebSocket {
    constructor(url, protocols) {
      super(url, protocols)
      this.__sentinelShellCandidate = shouldTrackUrl(String(url || ''))

      if (this.__sentinelShellCandidate) {
        upsertSession(this, {
          wsUrl: String(url || ''),
          protocol: protocolToString(protocols),
          connected: false,
          writable: true,
          terminalKind: 'websocket_shell',
        })
      }

      this.addEventListener('open', () => {
        const current = getSessionBySocket(this)
        if (this.__sentinelShellCandidate || current) {
          upsertSession(this, {
            wsUrl: String(url || ''),
            protocol: protocolToString(protocols),
            connected: true,
            writable: true,
            terminalKind: 'websocket_shell',
          })
        }
      })

      this.addEventListener('close', () => {
        const current = getSessionBySocket(this)
        if (!current) return
        removeSessionTracking(current.id, 'socket_closed')
      })

      this.addEventListener('message', event => {
        const payloadLooksInteractive = looksLikeShellPayload(event.data)
        const current = getSessionBySocket(this)
        if (!current && !this.__sentinelShellCandidate && !payloadLooksInteractive) {
          return
        }
        if (!current) {
          if (suppressedSockets.has(this)) {
            const root = findLikelyTerminalRoot()
            if (!root) {
              return
            }
            suppressedSockets.delete(this)
          }
          upsertSession(this, {
            wsUrl: String(url || ''),
            protocol: protocolToString(protocols),
            connected: true,
            writable: true,
            terminalKind: 'websocket_shell',
          })
        }
        emitFrame(this, 'in', event.data)
        const session = getSessionBySocket(this)
        if (session) {
          trackInboundForActiveWrite(session.id, event.data)
        }
      })
    }

    send(data) {
      const payloadLooksInteractive = looksLikeShellPayload(data)
      const current = getSessionBySocket(this)
      if (!current && (this.__sentinelShellCandidate || payloadLooksInteractive)) {
        if (suppressedSockets.has(this)) {
          const root = findLikelyTerminalRoot()
          if (!root) {
            return super.send(data)
          }
          suppressedSockets.delete(this)
        }
        upsertSession(this, {
          wsUrl: this.url,
          protocol: this.protocol,
          connected: true,
          writable: true,
          terminalKind: 'websocket_shell',
        })
      }
      if (getSessionBySocket(this)) {
        emitFrame(this, 'out', data)
      }
      return super.send(data)
    }
  }

  Object.defineProperty(SentinelWrappedWebSocket, 'CONNECTING', { value: NativeWebSocket.CONNECTING })
  Object.defineProperty(SentinelWrappedWebSocket, 'OPEN', { value: NativeWebSocket.OPEN })
  Object.defineProperty(SentinelWrappedWebSocket, 'CLOSING', { value: NativeWebSocket.CLOSING })
  Object.defineProperty(SentinelWrappedWebSocket, 'CLOSED', { value: NativeWebSocket.CLOSED })
  window.WebSocket = SentinelWrappedWebSocket
  interceptTerminalConstructor('Terminal')

  const rootLifecycleObserver = new MutationObserver(() => {
    scheduleRootLifecycleScan()
  })

  function startRootLifecycleObserver() {
    if (!document.documentElement) return
    rootLifecycleObserver.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['class', 'style', 'hidden', 'open', 'aria-hidden'],
    })
  }

  startRootLifecycleObserver()

  window.addEventListener('message', event => {
    if (event.source !== window) return
    if (event.data?.source !== 'sentinel-content-script') return
    if (event.data?.type !== 'sentinel:browser-shell-write') return

    const payload = event.data.payload || {}
    const normalizedSessionId = String(payload.sessionId || '').trim()
    if (!normalizedSessionId) {
      failQueuedWrite(payload, 'Session id is required')
      return
    }
    if (typeof payload.requestId !== 'string' || !payload.requestId.trim()) {
      failQueuedWrite(payload, 'Request id is required')
      return
    }
    if (typeof payload.inputText !== 'string' || !payload.inputText) {
      failQueuedWrite(payload, 'Input text is required')
      return
    }
    enqueueWrite({
      requestId: payload.requestId.trim(),
      sessionId: normalizedSessionId,
      inputText: payload.inputText,
    })
  })
})()
