import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ProgramDetailModal from './ProgramDetailModal.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('../../composables/useToast', () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  }),
}))

const program = {
  id: 'program-1',
  name: 'Example Program',
  organization: 'Example Org',
  platform: 'HackerOne',
  status: 'active',
  total_submissions: 0,
  total_earnings: 0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
}

function mountModal() {
  return mount(ProgramDetailModal, {
    props: {
      visible: true,
      program,
    },
    global: {
      stubs: {
        Teleport: true,
        Transition: false,
      },
    },
  })
}

describe('ProgramDetailModal', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockInvoke.mockResolvedValue([])
  })

  it('closes when the blank modal area is clicked', async () => {
    const wrapper = mountModal()

    await wrapper.get('.modal').trigger('click')

    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('keeps open when the dialog content is clicked', async () => {
    const wrapper = mountModal()

    await wrapper.get('.modal-box').trigger('click')

    expect(wrapper.emitted('close')).toBeUndefined()
  })
})
