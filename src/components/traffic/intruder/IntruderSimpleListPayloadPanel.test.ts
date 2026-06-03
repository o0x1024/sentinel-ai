import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, beforeEach, vi } from 'vitest'
import IntruderSimpleListPayloadPanel from './IntruderSimpleListPayloadPanel.vue'
import IntruderPayloadSourcePicker from './IntruderPayloadSourcePicker.vue'
import type { IntruderPayloadSet } from './types'

vi.mock('./IntruderPayloadSourcePicker.vue', () => ({
  default: {
    name: 'IntruderPayloadSourcePicker',
    props: ['open'],
    template: '<div />',
  },
}))

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('../trafficMessageViewTestMocks')
  return createVueI18nMock('zh-CN')
})

vi.mock('@/composables/useDialog', async () => {
  const { createDialogToastMock } = await import('../trafficMessageViewTestMocks')
  return createDialogToastMock()
})

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
}))

function createPayloadSet(overrides: Partial<IntruderPayloadSet> = {}): IntruderPayloadSet {
  return {
    id: 'payload-set-1',
    name: 'Payload set 1',
    payloadType: 'simpleList',
    payloadsText: 'alpha\nbeta',
    urlEncode: false,
    urlEncodeCharacters: '',
    dictionaryConfig: {
      sources: [],
      limit: 200,
      deduplicate: true,
    },
    pluginId: '',
    pluginPresetName: '',
    pluginConfig: '{}',
    filePath: '',
    bruteForceCharacterSet: 'abcdefghijklmnopqrstuvwxyz0123456789',
    bruteForceMinLength: 4,
    bruteForceMaxLength: 4,
    characterList: '',
    substitutionSource: '',
    substitutionRules: '',
    numberFrom: 0,
    numberTo: 0,
    numberStep: 1,
    numberPadWidth: 0,
    dateFrom: '',
    dateTo: '',
    dateStepDays: 1,
    dateFormat: 'yyyy-MM-dd',
    nullCount: 0,
    nullValue: '',
    usernameFirstNames: '',
    usernameLastNames: '',
    usernameFormats: '',
    ...overrides,
  }
}

function createPanelProps(overrides: Partial<IntruderPayloadSet> = {}) {
  const payloadSet = createPayloadSet(overrides)
  return {
    payloadSet,
    payloadSets: [payloadSet],
    attackType: 'sniper' as const,
    positions: [],
    requestText: 'POST /login HTTP/1.1\r\nHost: example.com\r\n\r\nusername=$admin$',
  }
}

describe('IntruderSimpleListPayloadPanel', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
  })

  it('renders payloads as table rows', () => {
    const wrapper = mount(IntruderSimpleListPayloadPanel, {
      props: {
        ...createPanelProps(),
      },
    })

    const rows = wrapper.findAll('[data-testid="simple-list-payload-row"]')
    expect(rows).toHaveLength(2)
    const inputs = wrapper.findAll('tbody input')
    expect((inputs[0]!.element as HTMLInputElement).value).toBe('alpha')
    expect((inputs[1]!.element as HTMLInputElement).value).toBe('beta')
  })

  it('appends clipboard payloads without opening an extra editor flow', async () => {
    global.testUtils.mockInvoke.mockResolvedValue({
      success: true,
      data: 'gamma\ndelta',
    })

    const wrapper = mount(IntruderSimpleListPayloadPanel, {
      props: {
        ...createPanelProps(),
      },
    })

    await wrapper.get('[data-testid="simple-list-paste"]').trigger('click')
    await flushPromises()

    expect(global.testUtils.mockInvoke).toHaveBeenCalledWith('read_traffic_clipboard_text')
    expect(wrapper.emitted('update')).toEqual([
      [{ payloadsText: 'alpha\nbeta\ngamma\ndelta' }],
    ])
  })

  it('adds a single payload from the inline input row', async () => {
    const wrapper = mount(IntruderSimpleListPayloadPanel, {
      props: {
        ...createPanelProps(),
      },
    })

    await wrapper.get('[data-testid="simple-list-add-input"]').setValue('gamma')
    await wrapper.get('[data-testid="simple-list-add"]').trigger('click')

    expect(wrapper.emitted('update')).toEqual([
      [{ payloadsText: 'alpha\nbeta\ngamma' }],
    ])
  })

  it('opens the payload library modal from the trigger button', async () => {
    const wrapper = mount(IntruderSimpleListPayloadPanel, {
      props: {
        ...createPanelProps({ payloadsText: '' }),
      },
    })

    await wrapper.get('[data-testid="simple-list-open-library"]').trigger('click')
    await flushPromises()

    expect(wrapper.findComponent(IntruderPayloadSourcePicker).props('open')).toBe(true)
  })

  it('imports payloads from the payload library', async () => {
    const wrapper = mount(IntruderSimpleListPayloadPanel, {
      props: {
        ...createPanelProps({ payloadsText: '' }),
      },
    })

    wrapper.findComponent(IntruderPayloadSourcePicker).vm.$emit('import', {
      mode: 'mergeDeduplicate',
      payloads: ['张三', '李四', '张三'],
      sourceRef: 'builtin:usernames-chinese',
    })
    await flushPromises()

    const emitted = wrapper.emitted('update')
    expect(emitted).toHaveLength(1)
    expect(emitted?.[0]?.[0]).toEqual({
      payloadsText: '张三\n李四',
    })
  })
})
