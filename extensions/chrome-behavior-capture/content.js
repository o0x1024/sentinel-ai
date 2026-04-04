(function sentinelBehaviorCapture() {
  const MAX_TEXT_LENGTH = 120
  const INPUT_EVENT_DEBOUNCE_MS = 600
  let inputTimer = null

  const emitEvents = (events) => {
    if (!Array.isArray(events) || events.length === 0) return
    chrome.runtime.sendMessage({
      type: 'sentinel:capture-events',
      events,
    })
  }

  const baseEvent = (eventType, extra = {}) => ({
    eventType,
    occurredAt: new Date().toISOString(),
    url: window.location.href,
    host: window.location.host,
    title: document.title,
    route: window.location.pathname + window.location.search + window.location.hash,
    ...extra,
  })

  const trimText = (raw) => {
    if (!raw) return ''
    const normalized = String(raw).trim().replace(/\s+/g, ' ')
    return normalized.length > MAX_TEXT_LENGTH
      ? normalized.slice(0, MAX_TEXT_LENGTH)
      : normalized
  }

  const describeElement = (element) => {
    if (!(element instanceof Element)) {
      return {}
    }
    return {
      text: trimText(
        element.getAttribute('aria-label')
          || element.getAttribute('title')
          || element.innerText
          || element.textContent
      ),
      role: trimText(
        element.getAttribute('role')
          || element.tagName.toLowerCase()
      ),
      selector: trimText(buildSelector(element)),
      inputName: trimText(
        element.getAttribute('name')
          || element.getAttribute('id')
          || element.getAttribute('placeholder')
      ),
    }
  }

  const buildSelector = (element) => {
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

  const emitPageLoad = () => {
    emitEvents([baseEvent('page_load')])
  }

  const emitRouteChange = () => {
    emitEvents([baseEvent('route_change')])
  }

  document.addEventListener('click', (event) => {
    const target = event.target instanceof Element ? event.target.closest('button, a, [role="button"], input, textarea, select') || event.target : null
    emitEvents([baseEvent('click', describeElement(target))])
  }, true)

  document.addEventListener('submit', (event) => {
    const target = event.target instanceof Element ? event.target : null
    emitEvents([baseEvent('submit', describeElement(target))])
  }, true)

  document.addEventListener('change', (event) => {
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

  const wrapHistoryMethod = (method) => {
    const original = history[method]
    history[method] = function wrappedHistoryMethod(...args) {
      const result = original.apply(this, args)
      emitRouteChange()
      return result
    }
  }

  wrapHistoryMethod('pushState')
  wrapHistoryMethod('replaceState')
  window.addEventListener('popstate', emitRouteChange)
  window.addEventListener('hashchange', emitRouteChange)
  window.addEventListener('load', emitPageLoad, { once: true })

  emitPageLoad()
})()
