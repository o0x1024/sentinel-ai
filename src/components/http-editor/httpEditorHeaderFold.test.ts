import { EditorState } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import { afterEach, describe, expect, it } from 'vitest'
import { createHeaderCollapseExtension } from './httpEditorHeaderFold'

let editorView: EditorView | null = null

afterEach(() => {
  editorView?.destroy()
  editorView = null
  document.body.innerHTML = ''
})

function mountEditor(doc: string, enabled = true) {
  const parent = document.createElement('div')
  document.body.appendChild(parent)
  editorView = new EditorView({
    state: EditorState.create({
      doc,
      extensions: [createHeaderCollapseExtension(enabled)],
    }),
    parent,
  })
  return parent
}

describe('httpEditorHeaderFold', () => {
  it('collapses HTTP headers without hiding the start line or body', () => {
    const parent = mountEditor(
      'GET /api HTTP/1.1\r\nHost: example.com\r\nCookie: a=b\r\nAccept: */*\r\n\r\n{"ok":true}',
    )

    expect(parent.textContent).toContain('GET /api HTTP/1.1')
    expect(parent.textContent).toContain('Headers collapsed (3)')
    expect(parent.textContent).toContain('{"ok":true}')
    expect(parent.textContent).not.toContain('Cookie: a=b')
  })

  it('leaves the message unchanged when header collapse is disabled', () => {
    const parent = mountEditor(
      'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Trace: 1\r\n\r\n{"ok":true}',
      false,
    )

    expect(parent.textContent).toContain('Content-Type: application/json')
    expect(parent.textContent).not.toContain('Headers collapsed')
  })
})
