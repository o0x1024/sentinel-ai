import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import ThinkingMessageBlock from './ThinkingMessageBlock.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === 'agent.thinkingTitle') return 'Thinking'
      if (key === 'agent.thinkingStats') {
        return `${String(params?.lines ?? 0)} lines / ${String(params?.chars ?? 0)} chars`
      }
      if (key === 'agent.statusRunning') return 'Running'
      if (key === 'agent.aiIsThinking') return 'AI is thinking...'
      return key
    },
  }),
}))

const longThinkingContent = Array.from(
  { length: 90 },
  (_, index) => `reasoning line ${index + 1}`
).join('\n')

describe('ThinkingMessageBlock', () => {
  it('uses an unlimited expanded view while thinking is streaming', () => {
    const wrapper = mount(ThinkingMessageBlock, {
      props: {
        content: longThinkingContent,
        status: 'streaming',
      },
    })

    const content = wrapper.get('pre')
    expect(content.isVisible()).toBe(true)
    expect(content.classes()).toContain('is-streaming')
    expect(content.classes()).not.toContain('is-limited-height')
    expect(wrapper.text()).toContain('90 lines')
    expect(wrapper.text()).toContain('Running')
  })

  it('collapses when streaming completes and uses limited height when reopened', async () => {
    const wrapper = mount(ThinkingMessageBlock, {
      props: {
        content: longThinkingContent,
        status: 'streaming',
      },
    })

    await wrapper.setProps({ status: 'complete' })
    await nextTick()

    expect(wrapper.get('pre').isVisible()).toBe(false)

    await wrapper.get('.thinking-header-button').trigger('click')
    await nextTick()

    const content = wrapper.get('pre')
    expect(wrapper.get('.thinking-header-button').attributes('aria-expanded')).toBe('true')
    expect(content.classes()).toContain('is-limited-height')
    expect(content.classes()).not.toContain('is-streaming')
    expect(wrapper.emitted('heightChanged')).toBeTruthy()
  })

  it('keeps completed thinking collapsed by default', async () => {
    const wrapper = mount(ThinkingMessageBlock, {
      props: {
        content: longThinkingContent,
        status: 'complete',
      },
    })

    expect(wrapper.get('pre').isVisible()).toBe(false)

    await wrapper.get('.thinking-header-button').trigger('click')
    await nextTick()

    const content = wrapper.get('pre')
    expect(wrapper.get('.thinking-header-button').attributes('aria-expanded')).toBe('true')
    expect(content.classes()).toContain('is-limited-height')
    expect(content.classes()).not.toContain('is-streaming')
  })
})
