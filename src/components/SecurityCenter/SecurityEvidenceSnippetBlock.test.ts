import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SecurityEvidenceSnippetBlock from './SecurityEvidenceSnippetBlock.vue'

describe('SecurityEvidenceSnippetBlock', () => {
  it('renders structured snippet entries as cards plus raw text', () => {
    const wrapper = mount(SecurityEvidenceSnippetBlock, {
      props: {
        snippet: "location=body | technique=error-based | probe=utm_source'",
      },
    })

    expect(wrapper.text()).toContain('Location')
    expect(wrapper.text()).toContain('Technique')
    expect(wrapper.text()).toContain('PoC')
    expect(wrapper.text()).toContain("utm_source'")
    expect(wrapper.findAll('div.rounded-lg.border').length).toBeGreaterThan(0)
    expect(wrapper.find('pre.bg-base-300').text()).toContain('location=body')
  })

  it('renders plain text summary as a single text block', () => {
    const wrapper = mount(SecurityEvidenceSnippetBlock, {
      props: {
        snippet: '系统 Agent 使用 conservative 策略完成验证，结果未确认。',
      },
    })

    expect(wrapper.findAll('div.rounded-lg.border')).toHaveLength(0)
    expect(wrapper.find('pre.bg-base-300').text()).toContain('系统 Agent 使用 conservative')
  })
})
