import { invoke } from '@tauri-apps/api/core'

export interface RagCollection {
  id: string
  name: string
  description?: string
  embedding_model: string
  chunk_size: number
  chunk_overlap: number
  created_at: string
  updated_at: string
  document_count: number
  chunk_count: number
}

export interface RagIngestRequest {
  collection_name: string
  source_path: string
  source_type: string
  metadata?: Record<string, any>
}

export interface RagIngestResponse {
  success: boolean
  message: string
  chunks_created?: number
  processing_time_ms?: number
}

export interface RagQueryRequest {
  collection_name: string
  query: string
  top_k?: number
  metadata_filter?: Record<string, any>
}

export interface RagQueryResponse {
  query: string
  results: RagQueryResult[]
  context: string
  processing_time_ms: number
}

export interface RagQueryResult {
  content: string
  score: number
  metadata: Record<string, string>
}

export interface RagStatus {
  initialized: boolean
  collections_count: number
  total_documents: number
  total_chunks: number
  embedding_provider?: string
  embedding_model?: string
}

export class RagService {
  /**
   * 初始化RAG服务
   */
  static async initialize(): Promise<boolean> {
    try {
      return await invoke('rag_initialize_service')
    } catch (error) {
      console.error('Failed to initialize RAG service:', error)
      throw error
    }
  }

  /**
   * 关闭RAG服务
   */
  static async shutdown(): Promise<boolean> {
    try {
      return await invoke('rag_shutdown_service')
    } catch (error) {
      console.error('Failed to shutdown RAG service:', error)
      throw error
    }
  }

  /**
   * 获取RAG系统状态
   */
  static async getStatus(): Promise<RagStatus> {
    try {
      return await invoke('rag_get_status')
    } catch (error) {
      console.error('Failed to get RAG status:', error)
      throw error
    }
  }

  /**
   * 获取支持的文件类型
   */
  static async getSupportedFileTypes(): Promise<string[]> {
    try {
      return await invoke('rag_get_supported_file_types')
    } catch (error) {
      console.error('Failed to get supported file types:', error)
      throw error
    }
  }

  /**
   * 导入数据源到RAG系统
   */
  static async ingestSource(
    filePath: string,
    collectionName?: string,
    metadata?: Record<string, string>
  ): Promise<RagIngestResponse> {
    try {
      return await invoke('rag_ingest_source', {
        filePath,
        collectionName,
        metadata
      })
    } catch (error) {
      console.error('Failed to ingest source:', error)
      throw error
    }
  }

  /**
   * 查询RAG系统
   */
  static async query(
    query: string,
    collectionName?: string,
    topK?: number,
    useMmr?: boolean,
    mmrLambda?: number,
    filters?: Record<string, string>
  ): Promise<RagQueryResponse> {
    try {
      return await invoke('rag_query', {
        query,
        collectionName,
        topK,
        useMmr,
        mmrLambda,
        filters
      })
    } catch (error) {
      console.error('Failed to query RAG:', error)
      throw error
    }
  }

  /**
   * 清空集合
   */
  static async clearCollection(collectionName: string): Promise<boolean> {
    try {
      return await invoke('rag_clear_collection', {
        collectionName
      })
    } catch (error) {
      console.error('Failed to clear collection:', error)
      throw error
    }
  }
}