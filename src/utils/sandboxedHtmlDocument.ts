const PREVIEW_CSP = [
  "default-src 'none'",
  "base-uri 'none'",
  "connect-src 'none'",
  "font-src data:",
  "form-action 'none'",
  "frame-src 'none'",
  "img-src data: blob:",
  "manifest-src 'none'",
  "media-src data: blob:",
  "object-src 'none'",
  "script-src 'unsafe-inline'",
  "style-src 'unsafe-inline'",
  "worker-src 'none'",
].join('; ')

const PREVIEW_HEAD = [
  '<meta charset="utf-8">',
  `<meta http-equiv="Content-Security-Policy" content="${PREVIEW_CSP}">`,
  '<base href="about:srcdoc">',
  '<style>html,body{margin:0;padding:0;min-height:100%;background:#fff;}</style>',
].join('')

const HEAD_TAG_PATTERN = /<head(\s[^>]*)?>/i
const HTML_TAG_PATTERN = /<html(\s[^>]*)?>/i
const DOCUMENT_TAG_PATTERN = /<!doctype|<html[\s>]|<head[\s>]|<body[\s>]/i

export function buildSandboxedHtmlDocument(html: string): string {
  const source = html || ''

  if (!DOCUMENT_TAG_PATTERN.test(source)) {
    return `<!doctype html><html><head>${PREVIEW_HEAD}</head><body>${source}</body></html>`
  }

  if (HEAD_TAG_PATTERN.test(source)) {
    return source.replace(HEAD_TAG_PATTERN, match => `${match}${PREVIEW_HEAD}`)
  }

  if (HTML_TAG_PATTERN.test(source)) {
    return source.replace(HTML_TAG_PATTERN, match => `${match}<head>${PREVIEW_HEAD}</head>`)
  }

  return `<!doctype html><html><head>${PREVIEW_HEAD}</head><body>${source}</body></html>`
}
