function resolveTargetElement(target: EventTarget | null): HTMLElement | null {
  if (target instanceof HTMLElement) {
    return target
  }

  if (target instanceof Node) {
    return target.parentElement
  }

  return null
}

export function isEditableKeyboardTarget(target: EventTarget | null): boolean {
  const element = resolveTargetElement(target)
  if (!element) {
    return false
  }

  if (element.closest('input, textarea, select, [contenteditable], .cm-editor, .cm-content, .cm-line, .cm-textfield')) {
    return true
  }

  return Boolean(element.isContentEditable)
}
