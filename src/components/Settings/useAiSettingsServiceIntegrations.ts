import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

type ConfigItem = { key: string; value: string }

const loadAiConfigItems = async () => {
  const items = await invoke('get_config', {
    request: { category: 'ai', key: null },
  }) as ConfigItem[]
  return new Map(items.map((item) => [item.key, item.value]))
}

export const useAiSettingsServiceIntegrations = () => {
  const statsView = ref<'provider' | 'model'>('provider')
  const detailedStats = ref<any[]>([])
  const tavilyApiKeyLocal = ref('')
  const tavilyMaxResultsLocal = ref(5)
  const aliyunApiKeyLocal = ref('')
  const aliyunDefaultModelLocal = ref('qwen-vl-plus')
  const testingAliyun = ref(false)

  const totalRequests = computed(() => detailedStats.value.length)

  const loadDetailedStats = async () => {
    try {
      const stats = await invoke('get_detailed_ai_usage_stats') as any[]
      detailedStats.value = stats || []
    } catch (error) {
      console.warn('Failed to load detailed AI usage stats', error)
    }
  }

  const loadTavilyConfig = async () => {
    try {
      const configMap = await loadAiConfigItems()
      tavilyApiKeyLocal.value = String(configMap.get('tavily_api_key') || '')
      const maxResults = Number(configMap.get('tavily_max_results') || 5)
      tavilyMaxResultsLocal.value = Number.isNaN(maxResults) ? 5 : Math.min(Math.max(maxResults, 1), 20)
    } catch (error) {
      console.warn('Failed to load Tavily config', error)
    }
  }

  const saveTavilyConfig = async () => {
    try {
      await invoke('save_config_batch', {
        configs: [
          {
            category: 'ai',
            key: 'tavily_api_key',
            value: tavilyApiKeyLocal.value || '',
            description: 'Tavily API key for web search',
            is_encrypted: true,
          },
          {
            category: 'ai',
            key: 'tavily_max_results',
            value: String(tavilyMaxResultsLocal.value || 5),
            description: 'Default max results for Tavily',
            is_encrypted: false,
          },
        ],
      })
    } catch (error) {
      console.error('Failed to save Tavily config', error)
    }
  }

  const loadAliyunConfig = async () => {
    try {
      const configMap = await loadAiConfigItems()
      aliyunApiKeyLocal.value = String(configMap.get('aliyun_dashscope_api_key') || '')
      aliyunDefaultModelLocal.value = String(configMap.get('aliyun_dashscope_model') || 'qwen-vl-plus')
    } catch (error) {
      console.warn('Failed to load Aliyun config', error)
    }
  }

  const saveAliyunConfig = async () => {
    try {
      await invoke('save_config_batch', {
        configs: [
          {
            category: 'ai',
            key: 'aliyun_dashscope_api_key',
            value: aliyunApiKeyLocal.value || '',
            description: 'Aliyun DashScope API key for file upload',
            is_encrypted: true,
          },
          {
            category: 'ai',
            key: 'aliyun_dashscope_model',
            value: aliyunDefaultModelLocal.value || 'qwen-vl-plus',
            description: 'Default model for DashScope upload',
            is_encrypted: false,
          },
        ],
      })
    } catch (error) {
      console.error('Failed to save Aliyun config', error)
    }
  }

  const testAliyunConnection = async () => {
    if (!aliyunApiKeyLocal.value) {
      alert('请先输入 DashScope API Key')
      return
    }

    testingAliyun.value = true
    try {
      const result = await invoke('test_aliyun_dashscope_connection', {
        apiKey: aliyunApiKeyLocal.value,
        model: aliyunDefaultModelLocal.value || 'qwen-vl-plus',
      })
      alert(result ? '连接成功！' : '连接失败，请检查 API Key')
    } catch (error: any) {
      alert(`连接测试失败: ${error?.message || error}`)
    } finally {
      testingAliyun.value = false
    }
  }

  const saveServiceConfigs = async () => {
    await saveTavilyConfig()
    await saveAliyunConfig()
  }

  onMounted(() => {
    loadTavilyConfig()
    loadAliyunConfig()
    loadDetailedStats()
  })

  return {
    aliyunApiKeyLocal,
    aliyunDefaultModelLocal,
    detailedStats,
    loadDetailedStats,
    saveServiceConfigs,
    statsView,
    tavilyApiKeyLocal,
    tavilyMaxResultsLocal,
    testAliyunConnection,
    testingAliyun,
    totalRequests,
  }
}
