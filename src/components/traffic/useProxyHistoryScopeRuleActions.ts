import { computed, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import { buildTrafficContextSubmenu } from './trafficContextSubmenuSupport'
import type { ProxyConfigState, ProxyScopeRule } from './proxyConfigurationTypes'
import type { ProxyRequest } from './proxyHistoryTypes'
import {
  appendScopeRuleToProxyConfig,
  buildScopeRuleFromProxyRequest,
  cloneProxyScopeRule,
  type ScopeRuleDialogSavePayload,
  type TrafficScopeRuleDialogHandle,
  type TrafficScopeRuleMode,
} from './trafficScopeRuleActions'

type CommandResponse<T> = {
  success?: boolean
  data?: T
  error?: string
}

type ContextMenuState = {
  request: ProxyRequest | null
}

type UseProxyHistoryScopeRuleActionsOptions = {
  contextMenu: Ref<ContextMenuState>
  scopeIncludeRules: Ref<ProxyScopeRule[]>
  scopeExcludeRules: Ref<ProxyScopeRule[]>
  hideContextMenu: () => void
  t: (key: string, params?: Record<string, unknown>) => string
}

export function useProxyHistoryScopeRuleActions(options: UseProxyHistoryScopeRuleActionsOptions) {
  const scopeRuleDialogRef = ref<TrafficScopeRuleDialogHandle | null>(null)

  function openScopeRuleDialog(mode: TrafficScopeRuleMode) {
    const request = options.contextMenu.value.request
    options.hideContextMenu()
    if (!request) {
      return
    }

    try {
      scopeRuleDialogRef.value?.open({
        mode,
        rule: buildScopeRuleFromProxyRequest(request),
      })
    } catch (error) {
      console.error('Failed to create proxy scope rule from history request:', error)
      dialog.toast.error(options.t('trafficAnalysis.history.errors.invalidScopeRuleUrl'))
    }
  }

  const historyScopeSubmenu = computed(() =>
    buildTrafficContextSubmenu({
      key: 'history-scope',
      triggerLabelKey: 'addToScope',
      triggerIconClass: 'fas fa-bullseye text-accent',
      items: [
        {
          key: 'addToInScope',
          iconClass: 'fas fa-circle-check text-xs text-success',
          labelKey: 'addToInScope',
          onClick: () => openScopeRuleDialog('include'),
        },
        {
          key: 'addToOutOfScope',
          iconClass: 'fas fa-ban text-xs text-error',
          labelKey: 'addToOutOfScope',
          onClick: () => openScopeRuleDialog('exclude'),
        },
      ],
    }),
  )

  async function saveScopeRuleFromDialog(payload: ScopeRuleDialogSavePayload) {
    try {
      const response = await invoke<CommandResponse<ProxyConfigState>>('get_proxy_config')
      if (!response?.success || !response.data) {
        throw new Error(response?.error || 'Failed to load proxy configuration')
      }

      const nextConfig = appendScopeRuleToProxyConfig(response.data, payload.mode, payload.rule)
      const saveResponse = await invoke<CommandResponse<void>>('save_proxy_config', {
        config: nextConfig,
      })
      if (!saveResponse?.success) {
        throw new Error(saveResponse?.error || 'Failed to save proxy configuration')
      }

      options.scopeIncludeRules.value = nextConfig.scope_include_rules.map(cloneProxyScopeRule)
      options.scopeExcludeRules.value = nextConfig.scope_exclude_rules.map(cloneProxyScopeRule)
      const targetKey = payload.mode === 'include'
        ? 'trafficAnalysis.proxyConfiguration.scopeIncludeTitle'
        : 'trafficAnalysis.proxyConfiguration.scopeExcludeTitle'
      dialog.toast.success(options.t('trafficAnalysis.history.messages.scopeRuleAdded', {
        target: options.t(targetKey),
      }))
    } catch (error) {
      console.error('Failed to save proxy scope rule from history request:', error)
      dialog.toast.error(options.t('trafficAnalysis.history.errors.scopeRuleSaveFailed', {
        error: String(error),
      }))
    }
  }

  return {
    historyScopeSubmenu,
    saveScopeRuleFromDialog,
    scopeRuleDialogRef,
  }
}
