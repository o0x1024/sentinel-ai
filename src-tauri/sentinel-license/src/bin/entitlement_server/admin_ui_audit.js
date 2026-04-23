function currentAuditLimit() {
  return Number(el.auditLimitSelect.value || 50)
}

async function loadAudit() {
  const filter = el.auditCustomerFilter.value.trim()
  const eventType = el.auditEventTypeFilter.value.trim()
  const query = new URLSearchParams()

  if (filter) query.set('customer_id', filter)
  if (eventType) query.set('event_type', eventType)
  query.set('limit', String(currentAuditLimit()))
  query.set('offset', String(state.auditOffset))

  const payload = await requestJson(`/api/admin/audit?${query.toString()}`)
  const events = payload.events || []
  const currentPage = Math.floor(state.auditOffset / currentAuditLimit()) + 1

  el.auditPagePill.textContent = `第 ${currentPage} 页`
  el.auditList.innerHTML = events.length
    ? events
        .map(
          event => `
        <article class="list-item">
          <header>
            <strong>${event.event_type}</strong>
            <span class="pill">${formatTs(event.created_at)}</span>
          </header>
          <div class="muted">Actor: ${event.actor}</div>
          <div class="muted">Customer: ${event.customer_id || '—'}</div>
          <div class="muted">Machine: <span class="mono">${event.machine_id || '—'}</span></div>
          <div class="audit-body">${event.details || '—'}</div>
          <pre class="mono audit-metadata">${event.metadata ? JSON.stringify(event.metadata, null, 2) : ''}</pre>
        </article>
      `,
        )
        .join('')
    : '<div class="list-item muted">暂无审计事件。</div>'
}

function bindAuditEventHandlers() {
  document.getElementById('auditPrevBtn').addEventListener('click', () => {
    state.auditOffset = Math.max(0, state.auditOffset - currentAuditLimit())
    loadAudit().catch(err => handleActionError(el.authStatus, err))
  })

  document.getElementById('auditNextBtn').addEventListener('click', () => {
    state.auditOffset += currentAuditLimit()
    loadAudit().catch(err => handleActionError(el.authStatus, err))
  })

  el.auditLimitSelect.addEventListener('change', () => {
    state.auditOffset = 0
    loadAudit().catch(err => handleActionError(el.authStatus, err))
  })

  el.auditCustomerFilter.addEventListener('change', () => {
    state.auditOffset = 0
  })

  el.auditEventTypeFilter.addEventListener('change', () => {
    state.auditOffset = 0
  })
}
