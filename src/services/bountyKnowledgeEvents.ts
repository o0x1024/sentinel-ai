export const BOUNTY_KNOWLEDGE_UPDATED_EVENT = 'sentinel:bounty-knowledge-updated'

export function emitBountyKnowledgeUpdated() {
  if (typeof window === 'undefined') {
    return
  }

  window.dispatchEvent(new CustomEvent(BOUNTY_KNOWLEDGE_UPDATED_EVENT))
}
