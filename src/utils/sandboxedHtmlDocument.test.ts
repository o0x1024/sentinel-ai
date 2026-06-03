import { describe, expect, it } from 'vitest'

import { buildSandboxedHtmlDocument } from './sandboxedHtmlDocument'

describe('sandboxedHtmlDocument', () => {
  it('wraps html fragments in a sandboxed document shell', () => {
    const result = buildSandboxedHtmlDocument('<div>Hello</div>')

    expect(result).toContain('<!doctype html>')
    expect(result).toContain('<meta http-equiv="Content-Security-Policy"')
    expect(result).toContain('<base href="about:srcdoc">')
    expect(result).toContain('<body><div>Hello</div></body>')
  })

  it('injects preview controls into an existing head', () => {
    const result = buildSandboxedHtmlDocument(
      '<!doctype html><html><head><title>Preview</title></head><body>Hi</body></html>',
    )

    expect(result).toContain('<head><meta charset="utf-8">')
    expect(result).toContain('<title>Preview</title>')
    expect(result).toContain('<body>Hi</body>')
  })

  it('creates a head when the source document omits one', () => {
    const result = buildSandboxedHtmlDocument('<html><body><script>1</script></body></html>')

    expect(result).toContain('<html><head><meta charset="utf-8">')
    expect(result).toContain("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'")
    expect(result).toContain('<body><script>1</script></body>')
  })
})
