import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SecurityEvidenceHighlightTermList from './SecurityEvidenceHighlightTermList.vue'

describe('SecurityEvidenceHighlightTermList', () => {
  it('renders short terms as badges', () => {
    const wrapper = mount(SecurityEvidenceHighlightTermList, {
      props: {
        label: 'PoC',
        tone: 'request',
        terms: ["utm_source'"],
      },
    })

    expect(wrapper.find('span.badge-warning').text()).toBe('PoC')
    expect(wrapper.findAll('span.badge-outline')).toHaveLength(1)
    expect(wrapper.find('pre').exists()).toBe(false)
  })

  it('renders long payloads as multiline code blocks', () => {
    const wrapper = mount(SecurityEvidenceHighlightTermList, {
      props: {
        label: 'PoC',
        tone: 'request',
        terms: [
          'query($Params1:CheckServiceLinkedRoleReq!){Bff_CheckServiceLinkedRole100:Bff_CheckServiceLinkedRole(Params:$Params1){__typename}}',
        ],
      },
    })

    expect(wrapper.find('span.badge-warning').text()).toBe('PoC')
    expect(wrapper.findAll('span.badge-outline')).toHaveLength(0)
    expect(wrapper.find('pre').text()).toContain('query($Params1:CheckServiceLinkedRoleReq!)')
  })
})
