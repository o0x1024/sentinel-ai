import { forceParsing } from '@codemirror/language'
import type { EditorView } from '@codemirror/view'
import { shouldHighlightTrafficMessageSyntax, type TrafficMessageType } from '@/components/traffic/trafficDisplaySettings'

const SYNTAX_HIGHLIGHT_MEASURE_KEY = Symbol('http-editor-syntax-highlight-measure')
const FULL_SYNTAX_PARSE_DOC_LIMIT = 250_000
const SYNTAX_PARSE_LOOKAHEAD = 24_000

interface HttpEditorSyntaxWarmupOptions {
  getEditorView: () => EditorView | null
  getEditorContainer: () => HTMLElement | undefined
  getMessageType: () => TrafficMessageType
}

export function useHttpEditorSyntaxWarmup(options: HttpEditorSyntaxWarmupOptions) {
  let pendingSyntaxForceFrames = 0
  let pendingSyntaxForceAnimationFrame: number | null = null
  let pendingSyntaxParseAnimationFrame: number | null = null
  let editorResizeObserver: ResizeObserver | null = null

  function forceVisibleSyntaxHighlight(retries = 4) {
    const editorView = options.getEditorView()
    if (!editorView || !shouldHighlightTrafficMessageSyntax(options.getMessageType())) return

    pendingSyntaxForceFrames = Math.max(retries, 1)

    if (pendingSyntaxForceAnimationFrame !== null) {
      cancelAnimationFrame(pendingSyntaxForceAnimationFrame)
    }

    const applyPendingSyntaxForce = () => {
      const currentEditorView = options.getEditorView()
      if (!currentEditorView) {
        pendingSyntaxForceAnimationFrame = null
        return
      }

      currentEditorView.requestMeasure({
        key: SYNTAX_HIGHLIGHT_MEASURE_KEY,
        read: (view) => ({
          viewportTo: view.viewport.to,
          docLength: view.state.doc.length,
        }),
        write: ({ viewportTo, docLength }) => {
          const parseTarget = docLength <= FULL_SYNTAX_PARSE_DOC_LIMIT
            ? docLength
            : Math.min(docLength, viewportTo + SYNTAX_PARSE_LOOKAHEAD)

          // forceParsing may dispatch an internal state update. Running it inside
          // CodeMirror's measure/write phase causes a re-entrant update crash.
          if (pendingSyntaxParseAnimationFrame !== null) {
            cancelAnimationFrame(pendingSyntaxParseAnimationFrame)
          }

          pendingSyntaxParseAnimationFrame = requestAnimationFrame(() => {
            pendingSyntaxParseAnimationFrame = null
            const latestEditorView = options.getEditorView()
            if (!latestEditorView) return

            forceParsing(
              latestEditorView,
              parseTarget,
              docLength <= FULL_SYNTAX_PARSE_DOC_LIMIT ? 300 : 200,
            )
          })
        },
      })

      pendingSyntaxForceFrames -= 1
      if (pendingSyntaxForceFrames <= 0) {
        pendingSyntaxForceAnimationFrame = null
        return
      }

      pendingSyntaxForceAnimationFrame = requestAnimationFrame(applyPendingSyntaxForce)
    }

    pendingSyntaxForceAnimationFrame = requestAnimationFrame(applyPendingSyntaxForce)
  }

  function cancelPendingSyntaxForce() {
    if (pendingSyntaxForceAnimationFrame !== null) {
      cancelAnimationFrame(pendingSyntaxForceAnimationFrame)
      pendingSyntaxForceAnimationFrame = null
    }
    if (pendingSyntaxParseAnimationFrame !== null) {
      cancelAnimationFrame(pendingSyntaxParseAnimationFrame)
      pendingSyntaxParseAnimationFrame = null
    }
    pendingSyntaxForceFrames = 0
  }

  function observeEditorContainer() {
    const editorContainer = options.getEditorContainer()
    if (!editorContainer) return

    disconnectEditorResizeObserver()
    editorResizeObserver = new ResizeObserver(() => {
      forceVisibleSyntaxHighlight(3)
    })
    editorResizeObserver.observe(editorContainer)
  }

  function disconnectEditorResizeObserver() {
    if (editorResizeObserver) {
      editorResizeObserver.disconnect()
      editorResizeObserver = null
    }
  }

  return {
    forceVisibleSyntaxHighlight,
    cancelPendingSyntaxForce,
    observeEditorContainer,
    disconnectEditorResizeObserver,
  }
}
