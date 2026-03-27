import { ref } from 'vue'

export interface CollectionStats {
  totalDocuments: number
  totalChunks: number
}

export interface CollectionDetails {
  documents: any[]
  chunks: any[]
  stats: CollectionStats
}

export const isAgentMemoryCollection = (collection: any) => collection?.name === 'agent_memory'

export const isCollectionActive = (collection: any) =>
  isAgentMemoryCollection(collection) || !!collection?.is_active

export const useRagManagementToast = () => {
  const toast = ref({
    show: false,
    message: '',
    type: 'info',
  })

  const showToast = (message: string, type: string = 'info') => {
    toast.value = { show: true, message, type }
    setTimeout(() => {
      toast.value.show = false
    }, 3000)
  }

  return {
    toast,
    showToast,
  }
}

export const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

export const formatBytes = (bytes: number) => {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let value = bytes
  while (value >= 1024 && i < units.length - 1) {
    value = value / 1024
    i++
  }
  return `${value.toFixed(1)} ${units[i]}`
}
