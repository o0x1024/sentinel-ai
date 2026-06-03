import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TrafficPluginRuntimePolicyCard from './TrafficPluginRuntimePolicyCard.vue'
import { createDefaultTrafficPluginRuntimeSettings } from './proxyConfigurationTypes'

function mountCard() {
  const policy = createDefaultTrafficPluginRuntimeSettings().activeProbe
  const wrapper = mount(TrafficPluginRuntimePolicyCard, {
    props: {
      title: 'Active Probe',
      description: 'Runtime policy',
      delayLabel: 'Host cooldown',
      policy,
      delayField: 'minHostCooldownMs',
    },
    global: {
      mocks: {
        $t: (key: string) => key,
      },
    },
  })

  return { wrapper, policy }
}

describe('TrafficPluginRuntimePolicyCard', () => {
  it('emits policy replacement when a scalar setting changes', async () => {
    const { wrapper, policy } = mountCard()
    const input = wrapper.findAll('input')[0]

    await input.setValue('123')

    const update = wrapper.emitted('update:policy')?.[0]?.[0] as typeof policy
    expect(update.maxQueueDepth).toBe(123)
    expect(update.jitterRange).toEqual(policy.jitterRange)
    expect(policy.maxQueueDepth).toBe(createDefaultTrafficPluginRuntimeSettings().activeProbe.maxQueueDepth)
  })

  it('emits a copied jitter range when jitter changes', async () => {
    const { wrapper, policy } = mountCard()
    const jitterMinInput = wrapper.findAll('input')[9]

    await jitterMinInput.setValue('77')

    const update = wrapper.emitted('update:policy')?.[0]?.[0] as typeof policy
    expect(update.jitterRange).toEqual([77, policy.jitterRange[1]])
    expect(update.jitterRange).not.toBe(policy.jitterRange)
    expect(policy.jitterRange[0]).toBe(createDefaultTrafficPluginRuntimeSettings().activeProbe.jitterRange[0])
  })
})
