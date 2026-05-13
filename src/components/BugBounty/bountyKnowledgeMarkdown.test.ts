import { describe, expect, it } from 'vitest'
import {
  formatBountyKnowledgeMarkdownSnippet,
  renderBountyKnowledgeMarkdown,
} from './bountyKnowledgeMarkdown'

describe('bountyKnowledgeMarkdown', () => {
  it('renders markdown content for knowledge notes', () => {
    const html = renderBountyKnowledgeMarkdown('## Steps\n\n- Find `token`\n- Re-test')

    expect(html).toContain('<h2>Steps</h2>')
    expect(html).toContain('<li>Find <code>token</code></li>')
  })

  it('sanitizes rendered markdown html', () => {
    const html = renderBountyKnowledgeMarkdown('<img src=x onerror="alert(1)">')

    expect(html).toContain('<img src="x">')
    expect(html).not.toContain('onerror')
  })

  it('keeps pasted image data URLs while sanitizing image attributes', () => {
    const html = renderBountyKnowledgeMarkdown('![proof](data:image/png;base64,AAAA "x")')

    expect(html).toContain('src="data:image/png;base64,AAAA"')
    expect(html).toContain('alt="proof"')
  })

  it('keeps pasted image data urls renderable', () => {
    const html = renderBountyKnowledgeMarkdown('![pasted](data:image/png;base64,abc)')

    expect(html).toContain('<img src="data:image/png;base64,abc"')
  })

  it('formats markdown as plain text for list snippets', () => {
    expect(formatBountyKnowledgeMarkdownSnippet('### OAuth\n\n[callback](https://example.com)')).toBe(
      'OAuth callback',
    )
  })
})
