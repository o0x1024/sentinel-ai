function resolveTargetElement(target: EventTarget | null): HTMLElement | null {
  if (target instanceof HTMLElement) {
    return target
  }

  if (target instanceof Node) {
    return target.parentElement
  }

  return null
}

function containsEditableKeyboardElement(target: EventTarget | null): boolean {
  const element = resolveTargetElement(target)
  if (!element) {
    return false
  }

  if (element.closest('input, textarea, select, [contenteditable], .cm-editor, .cm-content, .cm-line, .cm-textfield')) {
    return true
  }

  return Boolean(element.isContentEditable)
}

export function isEditableKeyboardTarget(target: EventTarget | null): boolean {
  return containsEditableKeyboardElement(target)
}

export function isEditableKeyboardEvent(event: KeyboardEvent): boolean {
  if (containsEditableKeyboardElement(event.target)) {
    return true
  }

  const path = event.composedPath()
  if (path.some((target) => containsEditableKeyboardElement(target))) {
    return true
  }

  return containsEditableKeyboardElement(document.activeElement)
}
