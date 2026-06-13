import { openImmersiveTrafficProxySettings } from '../immersiveTrafficDockState'

export type TrafficCodecSettingsTab = 'listeners' | 'analysis' | 'display'

const PENDING_SETTINGS_TAB_KEY = 'trafficAnalysis.codec.pendingSettingsTab'

export function openTrafficCodecRulesSettings() {
  sessionStorage.setItem(PENDING_SETTINGS_TAB_KEY, 'analysis')
  openImmersiveTrafficProxySettings()
}

export function consumeTrafficCodecPendingSettingsTab(): TrafficCodecSettingsTab | null {
  const tab = sessionStorage.getItem(PENDING_SETTINGS_TAB_KEY)
  if (!tab) return null
  sessionStorage.removeItem(PENDING_SETTINGS_TAB_KEY)
  if (tab === 'listeners' || tab === 'analysis' || tab === 'display') {
    return tab
  }
  return null
}
