import { Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet, type ViewUpdate } from '@codemirror/view'
import type { Extension } from '@codemirror/state'

class LineEndingWidget extends WidgetType {
  constructor(private readonly label: string) {
    super()
  }

  override eq(other: LineEndingWidget): boolean {
    return other.label === this.label
  }

  override toDOM(): HTMLElement {
    const marker = document.createElement('span')
    marker.className = 'cm-line-ending-indicator'
    marker.textContent = this.label
    marker.setAttribute('aria-hidden', 'true')
    return marker
  }
}

function buildLineEndingDecorations(view: EditorView, label: string): DecorationSet {
  const ranges = []

  for (const { from, to } of view.visibleRanges) {
    let line = view.state.doc.lineAt(from)

    while (line.from <= to) {
      if (line.number < view.state.doc.lines) {
        ranges.push(
          Decoration.widget({
            widget: new LineEndingWidget(label),
            side: 1,
          }).range(line.to),
        )
      }

      if (line.to >= to) {
        break
      }

      line = view.state.doc.line(line.number + 1)
    }
  }

  return Decoration.set(ranges, true)
}

export function getDetectedLineEndingLabel(content: string): string {
  if (content.includes('\r\n')) return '\\r\\n'
  if (content.includes('\r')) return '\\r'
  return '\\n'
}

export function createLineEndingIndicatorExtension(label = '\\n'): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet

      constructor(view: EditorView) {
        this.decorations = buildLineEndingDecorations(view, label)
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged) {
          this.decorations = buildLineEndingDecorations(update.view, label)
        }
      }
    },
    {
      decorations: (value) => value.decorations,
    },
  )
}
