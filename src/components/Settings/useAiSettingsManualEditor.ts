import { ref, watch, nextTick, onUnmounted, type ComputedRef } from 'vue'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { json } from '@codemirror/lang-json'
import { oneDark } from '@codemirror/theme-one-dark'
import { keymap } from '@codemirror/view'
import { defaultKeymap, indentWithTab } from '@codemirror/commands'
import { stripDerivedAiConfigFields } from '@/views/settingsAiSupport'

const buildDefaultProvider = (
  id: string,
  name: string,
  rigProvider: string,
  apiBase: string | null,
  extra: Record<string, unknown> = {},
) => ({
  id,
  provider: id,
  name,
  api_key: null,
  api_base: apiBase,
  organization: null,
  enabled: false,
  default_model: '',
  models: [],
  rig_provider: rigProvider,
  max_context_length: null,
  ...extra,
})

const buildDefaultAiConfig = () => ({
  providers: {
    Anthropic: buildDefaultProvider('anthropic', 'Anthropic', 'anthropic', 'https://api.anthropic.com'),
    OpenAI: buildDefaultProvider('openai', 'OpenAI', 'openai', 'https://api.openai.com/v1'),
    'Azure OpenAI': buildDefaultProvider('azure', 'Azure OpenAI', 'azure', null),
    Cohere: buildDefaultProvider('cohere', 'Cohere', 'cohere', 'https://api.cohere.ai'),
    DeepSeek: buildDefaultProvider('deepseek', 'DeepSeek', 'deepseek', 'https://api.deepseek.com/v1'),
    EternalAI: buildDefaultProvider('eternalai', 'EternalAI', 'eternalai', null),
    'Google Gemini': buildDefaultProvider('gemini', 'Google Gemini', 'gemini', null),
    Galadriel: buildDefaultProvider('galadriel', 'Galadriel', 'galadriel', null),
    Groq: buildDefaultProvider('groq', 'Groq', 'groq', 'https://api.groq.com/openai/v1'),
    Hyperbolic: buildDefaultProvider('hyperbolic', 'Hyperbolic', 'hyperbolic', 'https://api.hyperbolic.xyz/v1'),
    Mira: buildDefaultProvider('mira', 'Mira', 'mira', null),
    Moonshot: buildDefaultProvider('moonshot', 'Moonshot', 'moonshot', 'https://api.moonshot.cn/v1'),
    Ollama: buildDefaultProvider('ollama', 'Ollama', 'ollama', 'http://localhost:11434'),
    OpenRouter: buildDefaultProvider('openrouter', 'OpenRouter', 'openrouter', 'https://openrouter.ai/api/v1', {
      http_referer: null,
      x_title: null,
    }),
    Perplexity: buildDefaultProvider('perplexity', 'Perplexity', 'perplexity', 'https://api.perplexity.ai'),
    TogetherAI: buildDefaultProvider('togetherai', 'TogetherAI', 'togetherai', 'https://api.together.xyz/v1'),
    xAI: buildDefaultProvider('xai', 'xAI', 'xai', 'https://api.x.ai/v1'),
  },
})

export const useAiSettingsManualEditor = (params: {
  aiConfig: ComputedRef<any>
  emitApplyManualConfig: (config: any) => void
}) => {
  const useGuiMode = ref(true)
  const manualConfigText = ref('')
  const configError = ref('')
  const configValid = ref(false)
  const editorContainer = ref<HTMLDivElement | null>(null)
  const fullscreenEditorContainer = ref<HTMLDivElement | null>(null)
  const isFullscreen = ref(false)

  let editorView: EditorView | null = null
  let fullscreenEditorView: EditorView | null = null

  const validateConfigText = () => {
    configError.value = ''
    configValid.value = false

    if (!manualConfigText.value.trim()) return

    try {
      const parsed = JSON.parse(manualConfigText.value)
      if (typeof parsed !== 'object' || parsed === null) {
        configError.value = '配置必须是有效的 JSON 对象'
        return
      }
      if (!parsed.providers || typeof parsed.providers !== 'object') {
        configError.value = '配置必须包含 providers 对象'
        return
      }
      for (const [providerName, providerConfig] of Object.entries(parsed.providers)) {
        if (typeof providerConfig !== 'object' || providerConfig === null) {
          configError.value = `Provider "${providerName}" 必须是对象`
          return
        }
        const config = providerConfig as any
        if (typeof config.enabled !== 'boolean') {
          configError.value = `Provider "${providerName}" 缺少必需的 enabled 字段（布尔值）`
          return
        }
      }
      configValid.value = true
    } catch (error) {
      configError.value = `JSON 解析错误: ${(error as Error).message}`
    }
  }

  const createEditorState = (onDocChanged: (value: string) => void, content: string) =>
    EditorState.create({
      doc: content,
      extensions: [
        basicSetup,
        json(),
        oneDark,
        keymap.of([...defaultKeymap, indentWithTab]),
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onDocChanged(update.state.doc.toString())
          }
        }),
      ],
    })

  const updateEditorContent = (content: string) => {
    if (editorView) {
      const currentContent = editorView.state.doc.toString()
      if (currentContent !== content) {
        editorView.dispatch({ changes: { from: 0, to: currentContent.length, insert: content } })
      }
    }
    if (fullscreenEditorView) {
      const fullscreenContent = fullscreenEditorView.state.doc.toString()
      if (fullscreenContent !== content) {
        fullscreenEditorView.dispatch({ changes: { from: 0, to: fullscreenContent.length, insert: content } })
      }
    }
  }

  const initCodeMirror = () => {
    if (!editorContainer.value) return
    if (editorView) {
      editorView.destroy()
      editorView = null
    }
    editorContainer.value.innerHTML = ''
    editorView = new EditorView({
      state: createEditorState((value) => {
        manualConfigText.value = value
        validateConfigText()
      }, manualConfigText.value),
      parent: editorContainer.value,
    })
  }

  const initFullscreenEditor = () => {
    if (!fullscreenEditorContainer.value) return
    if (fullscreenEditorView) {
      fullscreenEditorView.destroy()
      fullscreenEditorView = null
    }
    fullscreenEditorContainer.value.innerHTML = ''
    fullscreenEditorView = new EditorView({
      state: createEditorState((value) => {
        manualConfigText.value = value
        validateConfigText()
        if (editorView) {
          const normalContent = editorView.state.doc.toString()
          if (normalContent !== value) {
            editorView.dispatch({ changes: { from: 0, to: normalContent.length, insert: value } })
          }
        }
      }, manualConfigText.value),
      parent: fullscreenEditorContainer.value,
    })
    fullscreenEditorView.focus()
  }

  const validateConfig = () => {
    validateConfigText()
  }

  const applyManualConfig = () => {
    if (configError.value) return
    try {
      params.emitApplyManualConfig(JSON.parse(manualConfigText.value))
    } catch (error) {
      configError.value = `应用配置失败: ${(error as Error).message}`
    }
  }

  const formatConfig = () => {
    if (!manualConfigText.value.trim()) return
    try {
      const formatted = JSON.stringify(JSON.parse(manualConfigText.value), null, 2)
      manualConfigText.value = formatted
      updateEditorContent(formatted)
      validateConfigText()
    } catch {
      // keep invalid json as-is
    }
  }

  const resetToDefault = () => {
    const formatted = JSON.stringify(buildDefaultAiConfig(), null, 2)
    manualConfigText.value = formatted
    updateEditorContent(formatted)
    validateConfigText()
  }

  const toggleFullscreen = async () => {
    isFullscreen.value = true
    await nextTick()
    initFullscreenEditor()
  }

  const exitFullscreen = () => {
    if (fullscreenEditorView) {
      fullscreenEditorView.destroy()
      fullscreenEditorView = null
    }
    isFullscreen.value = false
  }

  const applyAndExitFullscreen = () => {
    applyManualConfig()
    exitFullscreen()
  }

  watch(
    () => params.aiConfig.value,
    (newConfig) => {
      if (newConfig && !useGuiMode.value) {
        const newText = JSON.stringify(stripDerivedAiConfigFields(newConfig), null, 2)
        manualConfigText.value = newText
        updateEditorContent(newText)
        validateConfigText()
      }
    },
    { immediate: true, deep: true },
  )

  watch(useGuiMode, async (isGuiMode) => {
    if (!isGuiMode && params.aiConfig.value) {
      manualConfigText.value = JSON.stringify(stripDerivedAiConfigFields(params.aiConfig.value), null, 2)
      validateConfigText()
      await nextTick()
      initCodeMirror()
    } else if (isGuiMode && editorView) {
      editorView.destroy()
      editorView = null
    }
  })

  onUnmounted(() => {
    if (editorView) {
      editorView.destroy()
      editorView = null
    }
    if (fullscreenEditorView) {
      fullscreenEditorView.destroy()
      fullscreenEditorView = null
    }
  })

  return {
    applyAndExitFullscreen,
    applyManualConfig,
    configError,
    configValid,
    editorContainer,
    exitFullscreen,
    formatConfig,
    fullscreenEditorContainer,
    isFullscreen,
    manualConfigText,
    resetToDefault,
    toggleFullscreen,
    useGuiMode,
    validateConfig,
  }
}
