import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import ProgramsPanel from './ProgramsPanel.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

const programs = [
  {
    id: 'program-1',
    name: 'Example Program',
    organization: 'Example Org',
    platform: 'HackerOne',
    status: 'active',
    total_submissions: 3,
    total_earnings: 1200,
  },
]

function mountPanel() {
  return mount(ProgramsPanel, {
    props: {
      programs,
      loading: false,
    },
  })
}

describe('ProgramsPanel', () => {
  it('uses list view by default', () => {
    const wrapper = mountPanel()

    expect(wrapper.find('table').exists()).toBe(true)
    expect(wrapper.find('.card.bg-base-200').exists()).toBe(false)
    expect(wrapper.get('[aria-label="bugBounty.programs.listView"]').attributes('aria-pressed')).toBe('true')
  })

  it('can switch from the default list view to card view', async () => {
    const wrapper = mountPanel()

    await wrapper.get('[aria-label="bugBounty.programs.cardView"]').trigger('click')

    expect(wrapper.find('table').exists()).toBe(false)
    expect(wrapper.find('.card.bg-base-200').exists()).toBe(true)
    expect(wrapper.get('[aria-label="bugBounty.programs.cardView"]').attributes('aria-pressed')).toBe('true')
  })
})
