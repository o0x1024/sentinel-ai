import { describe, expect, it } from 'vitest'
import { isEditableKeyboardTarget } from './editableKeyboardTarget'

describe('isEditableKeyboardTarget', () => {
  it('treats CodeMirror content as editable', () => {
    const editor = document.createElement('div')
    editor.className = 'cm-editor'

    const content = document.createElement('div')
    content.className = 'cm-content'
    editor.appendChild(content)

    document.body.appendChild(editor)
    expect(isEditableKeyboardTarget(content)).toBe(true)
    document.body.removeChild(editor)
  })

  it('treats text inputs as editable', () => {
    const input = document.createElement('input')
    expect(isEditableKeyboardTarget(input)).toBe(true)
  })

  it('rejects generic containers', () => {
    const container = document.createElement('div')
    expect(isEditableKeyboardTarget(container)).toBe(false)
  })
})
