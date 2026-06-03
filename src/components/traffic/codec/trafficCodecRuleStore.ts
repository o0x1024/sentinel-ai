import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TrafficCodecRule } from './trafficCodecTypes'

const rules = ref<TrafficCodecRule[]>([])
const loaded = ref(false)
const loading = ref(false)

export function useTrafficCodecRuleStore() {
  async function loadRules() {
    if (loading.value) return
    loading.value = true
    try {
      const response = await invoke<{ data: TrafficCodecRule[] }>('codec_list_rules')
      rules.value = response.data ?? []
      loaded.value = true
    } finally {
      loading.value = false
    }
  }

  async function saveRule(rule: TrafficCodecRule) {
    await invoke('codec_save_rule', { rule })
    await loadRules()
  }

  async function deleteRule(id: string) {
    await invoke('codec_delete_rule', { id })
    await loadRules()
  }

  async function reorderRules(ids: string[]) {
    await invoke('codec_reorder_rules', { ids })
    await loadRules()
  }

  function ensureLoaded() {
    if (!loaded.value && !loading.value) {
      loadRules()
    }
  }

  return { rules, loaded, loading, loadRules, saveRule, deleteRule, reorderRules, ensureLoaded }
}
