import { EditorState, RangeSetBuilder, type Extension } from '@codemirror/state'
import { SearchQuery } from '@codemirror/search'
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from '@codemirror/view'

export interface SearchHighlightRange {
  from: number
  to: number
  selected: boolean
}

function selectionOverlapsRange(
  selection: { from: number; to: number },
  range: { from: number; to: number },
) {
  return (selection.from === range.from && selection.to === range.to)
    || (selection.from === selection.to && selection.from >= range.from && selection.from <= range.to)
    || (selection.from < range.to && selection.to > range.from)
}

export function collectSearchHighlightRanges(
  content: string,
  query: SearchQuery,
  selection: { from: number; to: number } = { from: 0, to: 0 },
): SearchHighlightRange[] {
  if (!query.search || !query.valid) {
    return []
  }

  const state = EditorState.create({ doc: content })
  const cursor = query.getCursor(state)
  const ranges: Array<{ from: number; to: number }> = []
  let selectedIndex = -1

  for (let next = cursor.next(); !next.done; next = cursor.next()) {
    const range = next.value
    if (range.from === range.to) {
      continue
    }

    if (selectedIndex < 0 && selectionOverlapsRange(selection, range)) {
      selectedIndex = ranges.length
    }

    ranges.push(range)
  }

  if (selectedIndex < 0 && ranges.length > 0) {
    selectedIndex = 0
  }

  return ranges.map((range, index) => ({
    ...range,
    selected: index === selectedIndex,
  }))
}

function buildSearchHighlightDecorations(
  view: EditorView,
  query: SearchQuery,
): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>()
  const selection = view.state.selection.main
  const ranges = collectSearchHighlightRanges(view.state.doc.toString(), query, {
    from: selection.from,
    to: selection.to,
  })

  for (const range of ranges) {
    builder.add(
      range.from,
      range.to,
      Decoration.mark({
        class: range.selected ? 'cm-searchMatch cm-searchMatch-selected' : 'cm-searchMatch',
      }),
    )
  }

  return builder.finish()
}

export function createSearchHighlightExtension(query: SearchQuery): Extension {
  if (!query.search || !query.valid) {
    return []
  }

  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet

      constructor(view: EditorView) {
        this.decorations = buildSearchHighlightDecorations(view, query)
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.selectionSet || update.viewportChanged) {
          this.decorations = buildSearchHighlightDecorations(update.view, query)
        }
      }
    },
    {
      decorations: (value) => value.decorations,
    },
  )
}
