import { invoke } from '@tauri-apps/api/core'

export interface RagConfig {
  database_path?: string
  chunk_size_chars: number
  chunk_overlap_chars: number
  top_k: number
  mmr_lambda: number
  batch_size: number
  max_concurrent: number
  embedding_provider: string
  embedding_model: string
  embedding_dimensions?: number
  embedding_api_key?: string
  embedding_base_url?: string
  reranking_provider?: string
  reranking_model?: string
  reranking_enabled: boolean
  similarity_threshold: number
  augmentation_enabled: boolean
}

export async function getRagConfig(): Promise<RagConfig> {
  return await invoke<RagConfig>('get_rag_config')
}

export async function saveRagConfig(config: Partial<RagConfig>): Promise<boolean> {
  // 读取当前配置并合并，避免丢失其他字段
  const current = await getRagConfig()
  const merged: RagConfig = { ...current, ...config }
  return await invoke<boolean>('save_rag_config', { config: merged })
}
