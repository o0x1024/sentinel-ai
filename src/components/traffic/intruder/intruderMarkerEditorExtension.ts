import { RangeSetBuilder, StateField, type EditorState } from '@codemirror/state'
import { Decoration, EditorView, keymap, type DecorationSet } from '@codemirror/view'
import { parseIntruderMarkerRanges } from './intruderMarkers'

const intruderMarkerDecoration = Decoration.mark({
  class: 'cm-intruder-marker',
})

function buildMarkerDecorations(state: EditorState): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>()
  const text = state.doc.toString()

  for (const range of parseIntruderMarkerRanges(text)) {
    builder.add(range.from, range.to, intruderMarkerDecoration)
  }

  return builder.finish()
}

const intruderMarkerField = StateField.define<DecorationSet>({
  create(state) {
    return buildMarkerDecorations(state)
  },
  update(_, transaction) {
    if (transaction.docChanged) {
      return buildMarkerDecorations(transaction.state)
    }
    return buildMarkerDecorations(transaction.state)
  },
  provide: field => [
    EditorView.decorations.from(field),
    EditorView.atomicRanges.of(view => view.state.field(field)),
  ],
})

function resolveMarkerDeletionRange(
  state: EditorState,
  direction: 'backward' | 'forward',
): { from: number; to: number } | null {
  const ranges = parseIntruderMarkerRanges(state.doc.toString())
  const selection = state.selection.main

  if (!selection.empty) {
    const overlapping = ranges.filter((range) => range.from < selection.to && range.to > selection.from)
    if (!overlapping.length) return null
    return {
      from: Math.min(selection.from, ...overlapping.map(range => range.from)),
      to: Math.max(selection.to, ...overlapping.map(range => range.to)),
    }
  }

  const position = selection.from
  const matched = direction === 'backward'
    ? ranges.find(range => position > range.from && position <= range.to)
    : ranges.find(range => position >= range.from && position < range.to)

  if (!matched) return null
  return { from: matched.from, to: matched.to }
}

function deleteIntruderMarker(direction: 'backward' | 'forward') {
  return (view: EditorView) => {
    const range = resolveMarkerDeletionRange(view.state, direction)
    if (!range) return false

    view.dispatch({
      changes: { from: range.from, to: range.to, insert: '' },
      selection: { anchor: range.from },
    })
    return true
  }
}

const intruderMarkerDeletionKeymap = keymap.of([
  { key: 'Backspace', run: deleteIntruderMarker('backward') },
  { key: 'Delete', run: deleteIntruderMarker('forward') },
])

const intruderMarkerInteractionHandlers = EditorView.domEventHandlers({
  dblclick(event, view) {
    const position = view.posAtCoords({ x: event.clientX, y: event.clientY })
    if (position == null) return false

    const matched = parseIntruderMarkerRanges(view.state.doc.toString())
      .find(range => position >= range.from && position < range.to)

    if (!matched) return false

    view.dispatch({
      selection: {
        anchor: matched.contentFrom,
        head: matched.contentTo,
      },
    })
    return true
  },
})

const intruderMarkerTheme = EditorView.theme({
  '.cm-intruder-marker': {
    backgroundColor: 'oklch(var(--wa) / 0.18)',
    color: 'oklch(var(--bc))',
    borderRadius: '0.2rem',
    boxShadow: 'inset 0 0 0 1px oklch(var(--wa) / 0.55)',
    fontWeight: '600',
  },
})

export function getIntruderMarkerEditorExtensions() {
  return [
    intruderMarkerField,
    intruderMarkerDeletionKeymap,
    intruderMarkerInteractionHandlers,
    intruderMarkerTheme,
  ]
}
