import { describe, expect, it } from 'vitest'
import { formatBountyFindingMarkdownList } from './bountyFindingMarkdown'

describe('bountyFindingMarkdown', () => {
  it('joins stored reproduction markdown without stripping formatting', () => {
    const markdown = '1. Open `/admin`\n2. Paste image\n\n![proof](data:image/png;base64,AAAA)'

    expect(formatBountyFindingMarkdownList(JSON.stringify([markdown]))).toBe(markdown)
  })

  it('joins multi-part stored steps as markdown blocks', () => {
    expect(formatBountyFindingMarkdownList(JSON.stringify(['Open page', '**Confirm** issue']))).toBe(
      'Open page\n\n**Confirm** issue',
    )
  })
})
