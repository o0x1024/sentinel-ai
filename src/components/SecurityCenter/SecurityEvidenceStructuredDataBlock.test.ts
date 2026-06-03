import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SecurityEvidenceStructuredDataBlock from './SecurityEvidenceStructuredDataBlock.vue'

describe('SecurityEvidenceStructuredDataBlock', () => {
  it('renders object metadata, viewer, and raw json', () => {
    const wrapper = mount(SecurityEvidenceStructuredDataBlock, {
      props: {
        data: {
          status: 'ok',
          nested: {
            count: 2,
          },
        },
      },
    })

    expect(wrapper.text()).toContain('Object')
    expect(wrapper.text()).toContain('2 keys')
    expect(wrapper.text()).toContain('原始 JSON')
    expect(wrapper.text()).toContain('status:')
    expect(wrapper.text()).toContain('"ok"')
  })

  it('renders array metadata', () => {
    const wrapper = mount(SecurityEvidenceStructuredDataBlock, {
      props: {
        data: [{ id: 1 }, { id: 2 }],
      },
    })

    expect(wrapper.text()).toContain('Array')
    expect(wrapper.text()).toContain('2 items')
  })
})
