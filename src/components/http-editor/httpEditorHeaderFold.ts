import { RangeSetBuilder, StateField } from '@codemirror/state'
import { Decoration, EditorView, WidgetType, type DecorationSet } from '@codemirror/view'
import type { EditorState, Extension } from '@codemirror/state'

class CollapsedHeaderWidget extends WidgetType {
  constructor(
    private readonly headerCount: number,
    private readonly label: string,
  ) {
    super()
  }

  override eq(other: CollapsedHeaderWidget): boolean {
    return other.headerCount === this.headerCount && other.label === this.label
  }

  override toDOM(): HTMLElement {
    const marker = document.createElement('div')
    marker.className = 'cm-http-collapsed-headers'
    marker.textContent = this.label
    marker.setAttribute('aria-label', marker.textContent)
    return marker
  }

  override ignoreEvent(): boolean {
    return false
  }
}

function findHeaderFoldRange(state: EditorState) {
  const { doc } = state
  if (doc.lines < 3) return null

  let separatorLineNumber = doc.lines + 1
  for (let lineNumber = 2; lineNumber <= doc.lines; lineNumber++) {
    const line = doc.line(lineNumber)
    if (line.text.trim() === '') {
      separatorLineNumber = lineNumber
      break
    }
  }

  const headerCount = Math.max(0, separatorLineNumber - 2)
  if (headerCount === 0) return null

  const firstHeaderLine = doc.line(2)
  const to = separatorLineNumber < doc.lines
    ? doc.line(separatorLineNumber + 1).from
    : doc.line(Math.min(separatorLineNumber, doc.lines)).to

  if (to <= firstHeaderLine.from) return null

  return {
    from: firstHeaderLine.from,
    to,
    headerCount,
  }
}

function buildHeaderFoldDecorations(state: EditorState, formatLabel: (count: number) => string): DecorationSet {
  const range = findHeaderFoldRange(state)
  if (!range) return Decoration.none

  const builder = new RangeSetBuilder<Decoration>()
  builder.add(
    range.from,
    range.to,
    Decoration.replace({
      widget: new CollapsedHeaderWidget(range.headerCount, formatLabel(range.headerCount)),
      block: true,
    }),
  )
  return builder.finish()
}

export function createHeaderCollapseExtension(
  enabled: boolean,
  formatLabel: (count: number) => string = count => `Headers collapsed (${count})`,
): Extension {
  if (!enabled) return []

  return StateField.define<DecorationSet>({
    create(state) {
      return buildHeaderFoldDecorations(state, formatLabel)
    },
    update(decorations, transaction) {
      return transaction.docChanged
        ? buildHeaderFoldDecorations(transaction.state, formatLabel)
        : decorations
    },
    provide: field => EditorView.decorations.from(field),
  })
}
