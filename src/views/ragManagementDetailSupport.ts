import type { CollectionDetails } from './ragManagementUiSupport'

export const buildCollectionDetails = (collection: any): CollectionDetails => {
  return {
    documents: [],
    chunks: [],
    stats: {
      totalDocuments: collection?.document_count || 0,
      totalChunks: collection?.chunk_count || 0,
    },
  }
}
