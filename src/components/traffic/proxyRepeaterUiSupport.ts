export function generateRepeaterId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export function getRepeaterStatusClass(status: number): string {
  if (status >= 200 && status < 300) return 'badge-success';
  if (status >= 300 && status < 400) return 'badge-info';
  if (status >= 400 && status < 500) return 'badge-warning';
  if (status >= 500) return 'badge-error';
  return 'badge-ghost';
}

export function formatRepeaterBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

export function measureRepeaterTextBytes(value: string): number {
  return new TextEncoder().encode(value).length;
}

export function formatRepeaterResponseMeta(responseText: string, responseTimeMs: number): string {
  return `${responseTimeMs} ms | ${formatRepeaterBytes(measureRepeaterTextBytes(responseText))}`;
}
