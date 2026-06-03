export const REPEATER_STORAGE_KEY_LAYOUT = 'proxyRepeater.layoutMode'
export const REPEATER_STORAGE_KEY_LEFT_WIDTH = 'proxyRepeater.leftPanelWidth'
export const REPEATER_STORAGE_KEY_TOP_HEIGHT = 'proxyRepeater.topPanelHeight'
export const REPEATER_STORAGE_KEY_TABS = 'proxyRepeater.tabs'

export function clearRepeaterTabsStorage() {
  try {
    localStorage.removeItem(REPEATER_STORAGE_KEY_TABS)
  } catch (error) {
    console.error('Failed to clear repeater tabs:', error)
  }
}
