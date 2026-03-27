import { invoke } from '@tauri-apps/api/core'

export const loadSupportedFileTypes = async (): Promise<string[]> => {
  const types = (await invoke('rag_get_supported_file_types')) as string[]
  return Array.isArray(types) && types.length > 0 ? types : []
}

export const loadRagQueryThreshold = async (): Promise<number | null> => {
  const config = (await invoke('get_rag_config')) as any
  if (config && typeof config.similarity_threshold === 'number') {
    return config.similarity_threshold
  }
  return null
}
