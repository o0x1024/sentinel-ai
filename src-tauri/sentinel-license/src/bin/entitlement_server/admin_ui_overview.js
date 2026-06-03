async function loadOverview() {
  const health = await requestJson('/healthz')
  el.healthMetrics.innerHTML = [
    metricCard('Customers', health.customer_count),
    metricCard('Devices', health.device_binding_count),
    metricCard('Audit Events', health.audit_event_count),
    metricCard('Admin Users', health.admin_user_count),
  ].join('')
}
