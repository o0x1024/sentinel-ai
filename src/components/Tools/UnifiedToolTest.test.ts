import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import UnifiedToolTest from './UnifiedToolTest.vue'

vi.mock('@/composables/useDialog', () => ({
  dialog: {
    toast: {
      success: vi.fn(),
      error: vi.fn(),
    },
  },
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (_key: string, fallback?: string) => fallback || _key,
  }),
}))

const AppDialogStub = defineComponent({
  name: 'AppDialog',
  template: '<div><slot /></div>',
})

const MonitorPluginParamsFieldStub = defineComponent({
  name: 'MonitorPluginParamsField',
  props: {
    field: {
      type: Object,
      required: true,
    },
    params: {
      type: Object,
      required: true,
    },
  },
  template: `
    <button
      class="mock-param-field"
      type="button"
      @click="params[field.name] = 'changed-from-form'"
    >
      {{ field.name }}
    </button>
  `,
})

const schema = {
  type: 'object',
  required: ['command'],
  properties: {
    command: {
      type: 'string',
      description: 'Command to run',
      default: 'pwd',
    },
    verbose: {
      type: 'boolean',
      description: 'Verbose output',
      default: false,
    },
  },
}

function mountDialog() {
  return mount(UnifiedToolTest, {
    props: {
      modelValue: false,
      toolName: 'shell',
      toolType: 'builtin',
      inputSchema: schema,
    },
    global: {
      stubs: {
        AppDialog: AppDialogStub,
        JsonViewer: true,
        MonitorPluginParamsField: MonitorPluginParamsFieldStub,
      },
      mocks: {
        $t: (_key: string, fallback?: string) => fallback,
      },
    },
  })
}

describe('UnifiedToolTest', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockInvoke.mockResolvedValue({
      success: true,
      output: { ok: true },
    })
  })

  it('starts parameter help collapsed and supports form/json input switching', async () => {
    const wrapper = mountDialog()
    await wrapper.setProps({ modelValue: true })

    const advancedButton = wrapper.findAll('button').find(button => button.text() === '高级选项')
    expect(advancedButton).toBeTruthy()
    await advancedButton!.trigger('click')

    const helpToggle = wrapper.find('.collapse > input[type="checkbox"]')
    expect((helpToggle.element as HTMLInputElement).checked).toBe(false)

    expect(wrapper.find('.mock-param-field').exists()).toBe(true)
    await wrapper.find('.mock-param-field').trigger('click')

    const jsonTab = wrapper.findAll('button').find(button => button.text() === 'JSON')
    expect(jsonTab).toBeTruthy()
    await jsonTab!.trigger('click')

    const textarea = wrapper.find('textarea')
    expect((textarea.element as HTMLTextAreaElement).value).toContain('changed-from-form')

    await textarea.setValue('{"command":"whoami","verbose":true}')

    const runButton = wrapper.findAll('button').find(button => button.text() === '运行测试')
    expect(runButton).toBeTruthy()
    await runButton!.trigger('click')
    await flushPromises()

    expect(global.testUtils.mockInvoke).toHaveBeenCalledWith('unified_execute_tool', {
      toolName: 'shell',
      inputs: {
        command: 'whoami',
        verbose: true,
      },
      context: null,
      timeout: 120,
    })
  })
})
