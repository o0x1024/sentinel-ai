import { isEditableKeyboardTarget } from '@/utils/editableKeyboardTarget'
import type { SecurityEvidenceTransferTarget } from './securityEvidenceTransferSupport'

export function resolveSecurityEvidenceTransferShortcut(
  event: Pick<
    KeyboardEvent,
    | 'altKey'
    | 'ctrlKey'
    | 'defaultPrevented'
    | 'isComposing'
    | 'key'
    | 'metaKey'
    | 'repeat'
    | 'shiftKey'
    | 'target'
  >,
): SecurityEvidenceTransferTarget | null {
  if (event.defaultPrevented || event.repeat || event.isComposing) {
    return null
  }

  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) {
    return null
  }

  if (isEditableKeyboardTarget(event.target)) {
    return null
  }

  const normalizedKey = event.key.toLowerCase()
  if (normalizedKey === 'r') {
    return 'draft'
  }

  if (normalizedKey === 'i') {
    return 'attackWorkspace'
  }

  return null
}
