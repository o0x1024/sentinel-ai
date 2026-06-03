import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SecurityEvidenceTextBlock from './SecurityEvidenceTextBlock.vue'

describe('SecurityEvidenceTextBlock', () => {
  it('renders plain multiline text', () => {
    const wrapper = mount(SecurityEvidenceTextBlock, {
      props: {
        text: 'line one\nline two',
      },
    })

    expect(wrapper.text()).toContain('line one')
    expect(wrapper.text()).toContain('line two')
    expect(wrapper.find('pre').classes()).toContain('break-words')
  })

  it('renders monospaced blocks for urls and code-like text', () => {
    const wrapper = mount(SecurityEvidenceTextBlock, {
      props: {
        text: 'https://example.com/path/to/resource?id=1',
        mono: true,
        size: 'xs',
      },
    })

    expect(wrapper.find('pre').classes()).toContain('font-mono')
    expect(wrapper.find('pre').classes()).toContain('break-all')
  })

  it('does not render empty text', () => {
    const wrapper = mount(SecurityEvidenceTextBlock, {
      props: {
        text: '   ',
      },
    })

    expect(wrapper.find('pre').exists()).toBe(false)
  })
})
