import { defineComponent, h, ref } from 'vue'

interface HttpMessageSurfaceStubOptions {
  responseSearchTestId?: string
}

export function createHttpMessageSurfaceStub(options: HttpMessageSurfaceStubOptions = {}) {
  return defineComponent({
    name: 'HttpMessageSurface',
    props: {
      modelValue: {
        type: String,
        default: '',
      },
      showSearchBar: {
        type: Boolean,
        default: false,
      },
      stateKey: {
        type: String,
        default: '',
      },
      readonly: {
        type: Boolean,
        default: false,
      },
      displayMode: {
        type: String,
        default: 'raw',
      },
    },
    setup(props) {
      const searchValue = ref('')

      return () => h('div', {
        class: 'http-message-surface-stub',
        'data-testid': 'http-surface',
        'data-state-key': props.stateKey,
        'data-readonly': String(props.readonly),
        'data-display-mode': props.displayMode,
      }, [
        options.responseSearchTestId && props.showSearchBar && props.stateKey.includes(':response:')
          ? h('input', {
              'data-testid': options.responseSearchTestId,
              value: searchValue.value,
              onInput: (event: Event) => {
                searchValue.value = (event.target as HTMLInputElement).value
              },
            })
          : null,
        h('div', {
          'data-testid': props.stateKey.includes(':response:') ? 'response-content' : 'request-content',
          'data-readonly': String(props.readonly),
          'data-display-mode': props.displayMode,
        }, props.modelValue),
      ])
    },
  })
}

export const TrafficMessageReaderStub = defineComponent({
  name: 'TrafficMessageReader',
  props: {
    modelValue: {
      type: String,
      default: '',
    },
    stateKey: {
      type: String,
      default: '',
    },
  },
  setup(props) {
    return () => h('div', {
      'data-testid': 'traffic-reader',
      'data-state-key': props.stateKey,
    }, props.modelValue)
  },
})

export function createTrafficMessageViewTabsStub(testIdPrefix = 'traffic-tab') {
  return defineComponent({
    name: 'TrafficMessageViewTabs',
    props: {
      modelValue: {
        type: String,
        default: 'pretty',
      },
      tabs: {
        type: Array as () => Array<{ value: string; label: string }>,
        default: () => [],
      },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
      return () => h('div', { class: 'traffic-message-view-tabs-stub' }, props.tabs.map(tab => h('button', {
        type: 'button',
        'data-testid': `${testIdPrefix}-${tab.value}`,
        'data-active': String(props.modelValue === tab.value),
        onClick: () => emit('update:modelValue', tab.value),
      }, tab.label)))
    },
  })
}

export const CodeDiffViewerStub = defineComponent({
  name: 'CodeDiffViewer',
  setup() {
    return () => h('div', { 'data-testid': 'code-diff-viewer' })
  },
})

export const AppDialogStub = defineComponent({
  name: 'AppDialog',
  setup(_, { slots }) {
    return () => h('div', slots.default?.())
  },
})
