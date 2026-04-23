import {
  trafficCategories,
  type PluginRecord,
} from '@/components/PluginManagement/types'

export function isTrafficAnalysisPluginRecord(plugin: PluginRecord) {
  if (plugin.metadata.main_category === 'traffic') {
    return true
  }

  if (plugin.metadata.category === 'traffic') {
    return true
  }

  return trafficCategories.includes(plugin.metadata.category)
}

export function sortTrafficAnalysisPlugins(plugins: PluginRecord[]) {
  return [...plugins].sort((left, right) => {
    const leftEnabled = left.status === 'Enabled' ? 1 : 0
    const rightEnabled = right.status === 'Enabled' ? 1 : 0
    if (leftEnabled !== rightEnabled) {
      return rightEnabled - leftEnabled
    }

    return left.metadata.name.localeCompare(right.metadata.name, 'zh-Hans-CN')
  })
}
