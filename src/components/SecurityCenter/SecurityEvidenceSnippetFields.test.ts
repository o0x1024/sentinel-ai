import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SecurityEvidenceSnippetFields from './SecurityEvidenceSnippetFields.vue'

describe('SecurityEvidenceSnippetFields', () => {
  it('renders compact target path as badge', () => {
    const wrapper = mount(SecurityEvidenceSnippetFields, {
      props: {
        location: 'body',
        technique: 'error-based',
        targetPath: 'query.user.id',
        referenceStatus: '200',
        probeStatus: '500',
      },
    })

    expect(wrapper.text()).toContain('Location: body')
    expect(wrapper.text()).toContain('Technique: error-based')
    expect(wrapper.text()).toContain('Reference: 200')
    expect(wrapper.text()).toContain('Probe: 500')
    expect(wrapper.findAll('span.badge-outline.font-mono')).toHaveLength(1)
    expect(wrapper.find('pre').exists()).toBe(false)
  })

  it('renders long target path as multiline code block', () => {
    const wrapper = mount(SecurityEvidenceSnippetFields, {
      props: {
        targetPath:
          'query.viewer.organization.members.edges[0].node.permissions.allowedActions[0].scope.resourceId',
      },
    })

    expect(wrapper.find('span.badge-outline').text()).toBe('Target Path')
    expect(wrapper.find('pre').text()).toContain('permissions.allowedActions')
  })
})
