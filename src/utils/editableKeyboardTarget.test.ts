import { describe, expect, it } from 'vitest'
import { isEditableKeyboardEvent, isEditableKeyboardTarget } from './editableKeyboardTarget'

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

  it('treats an event composed path through CodeMirror as editable', () => {
    const editor = document.createElement('div')
    editor.className = 'cm-editor'
    const scroller = document.createElement('div')
    editor.appendChild(scroller)

    const event = new KeyboardEvent('keydown', { key: 'Backspace' })
    Object.defineProperty(event, 'composedPath', {
      value: () => [scroller, editor, document.body, document],
    })

    expect(isEditableKeyboardEvent(event)).toBe(true)
  })

  it('uses the focused editable element when the keyboard target is generic', () => {
    const container = document.createElement('div')
    const input = document.createElement('input')
    document.body.append(container, input)
    input.focus()

    const event = new KeyboardEvent('keydown', { key: 'Backspace' })
    container.dispatchEvent(event)

    expect(isEditableKeyboardEvent(event)).toBe(true)
    document.body.removeChild(container)
    document.body.removeChild(input)
  })
})
