import { describe, expect, it } from 'vitest'
import {
  buildPastedImageReferenceMarkdown,
  collapseBountyKnowledgeImageContent,
  expandBountyKnowledgeImageContent,
  insertTextAtSelection,
} from './bountyKnowledgeImagePaste'

describe('bountyKnowledgeImagePaste', () => {
  it('builds compact markdown image reference syntax for pasted image data', () => {
    expect(buildPastedImageReferenceMarkdown('bounty-image-1', 'Pasted image 1')).toBe(
      '![Pasted image 1][bounty-image-1]',
    )
  })

  it('inserts image markdown at the textarea cursor with paragraph spacing', () => {
    const result = insertTextAtSelection('before\nafter', '![img](data:image/png;base64,abc)', 7, 7)

    expect(result.value).toBe('before\n![img](data:image/png;base64,abc)\n\nafter')
    expect(result.cursor).toBe('before\n![img](data:image/png;base64,abc)\n\n'.length)
  })

  it('replaces the selected text with pasted image markdown', () => {
    const result = insertTextAtSelection('before selected after', '![img](data:image/png;base64,abc)', 7, 15)

    expect(result.value).toBe('before \n\n![img](data:image/png;base64,abc)\n\n after')
  })

  it('collapses inline pasted image data urls into hidden references for editing', () => {
    const result = collapseBountyKnowledgeImageContent('before\n![shot](data:image/png;base64,abc)\nafter')

    expect(result.content).toBe('before\n![shot][bounty-image-1]\nafter')
    expect(result.references).toEqual([{ id: 'bounty-image-1', dataUrl: 'data:image/png;base64,abc' }])
  })

  it('expands only used hidden image references for preview and persistence', () => {
    const result = expandBountyKnowledgeImageContent('![shot][bounty-image-1]', [
      { id: 'bounty-image-1', dataUrl: 'data:image/png;base64,abc' },
      { id: 'bounty-image-2', dataUrl: 'data:image/png;base64,unused' },
    ])

    expect(result).toBe('![shot][bounty-image-1]\n\n[bounty-image-1]: data:image/png;base64,abc')
  })
})
