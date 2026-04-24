import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SecurityEvidenceSnippetPreview from './SecurityEvidenceSnippetPreview.vue'

describe('SecurityEvidenceSnippetPreview', () => {
  it('renders structured snippet entries as cards', () => {
    const wrapper = mount(SecurityEvidenceSnippetPreview, {
      props: {
        evidenceSnippet:
          'location=body | technique=error-based | probe=query($Params1:CheckServiceLinkedRoleReq!){__typename}',
      },
    })

    expect(wrapper.text()).toContain('Location')
    expect(wrapper.text()).toContain('Technique')
    expect(wrapper.text()).toContain('PoC')
    expect(wrapper.text()).toContain('原始文本')
    expect(wrapper.findAll('div.rounded-lg.border').length).toBeGreaterThan(0)
  })

  it('renders plain snippets as raw text only', () => {
    const wrapper = mount(SecurityEvidenceSnippetPreview, {
      props: {
        evidenceSnippet: 'plain text snippet',
      },
    })

    expect(wrapper.text()).toContain('证据片段')
    expect(wrapper.text()).toContain('plain text snippet')
    expect(wrapper.findAll('div.rounded-lg.border')).toHaveLength(0)
  })
})
