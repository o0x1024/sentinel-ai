import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useHttpEditorSyntaxWarmup } from './useHttpEditorSyntaxWarmup'

const { forceParsingMock } = vi.hoisted(() => ({
  forceParsingMock: vi.fn(),
}))

vi.mock('@codemirror/language', () => ({
  forceParsing: forceParsingMock,
}))

describe('useHttpEditorSyntaxWarmup', () => {
  const queuedFrames = new Map<number, FrameRequestCallback>()
  let nextFrameId = 1

  function flushNextFrame() {
    const [frameId, callback] = queuedFrames.entries().next().value ?? []
    if (!frameId || !callback) {
      throw new Error('No animation frame queued')
    }
    queuedFrames.delete(frameId)
    callback(performance.now())
  }

  beforeEach(() => {
    queuedFrames.clear()
    nextFrameId = 1
    forceParsingMock.mockReset()

    vi.stubGlobal('requestAnimationFrame', ((callback: FrameRequestCallback) => {
      const frameId = nextFrameId
      nextFrameId += 1
      queuedFrames.set(frameId, callback)
      return frameId
    }) as typeof window.requestAnimationFrame)

    vi.stubGlobal('cancelAnimationFrame', ((frameId: number) => {
      queuedFrames.delete(frameId)
    }) as typeof window.cancelAnimationFrame)
  })

  it('defers forceParsing until after the measure write phase completes', () => {
    const editorView = {
      viewport: { to: 128 },
      state: { doc: { length: 512 } },
      requestMeasure: vi.fn(({ read, write }: any) => {
        const measurement = read(editorView)
        write(measurement, editorView)
      }),
    }

    const syntaxWarmup = useHttpEditorSyntaxWarmup({
      getEditorView: () => editorView as any,
      getEditorContainer: () => undefined,
      getMessageType: () => 'request',
    })

    syntaxWarmup.forceVisibleSyntaxHighlight(1)

    expect(editorView.requestMeasure).not.toHaveBeenCalled()
    expect(forceParsingMock).not.toHaveBeenCalled()

    flushNextFrame()

    expect(editorView.requestMeasure).toHaveBeenCalledTimes(1)
    expect(forceParsingMock).not.toHaveBeenCalled()

    flushNextFrame()

    expect(forceParsingMock).toHaveBeenCalledTimes(1)
    expect(forceParsingMock).toHaveBeenCalledWith(editorView, 512, 300)
  })

  it('cancels a queued parse when syntax warmup is cleared', () => {
    const editorView = {
      viewport: { to: 64 },
      state: { doc: { length: 256 } },
      requestMeasure: vi.fn(({ read, write }: any) => {
        const measurement = read(editorView)
        write(measurement, editorView)
      }),
    }

    const syntaxWarmup = useHttpEditorSyntaxWarmup({
      getEditorView: () => editorView as any,
      getEditorContainer: () => undefined,
      getMessageType: () => 'response',
    })

    syntaxWarmup.forceVisibleSyntaxHighlight(1)
    flushNextFrame()
    syntaxWarmup.cancelPendingSyntaxForce()

    expect(queuedFrames.size).toBe(0)
    expect(forceParsingMock).not.toHaveBeenCalled()
  })
})
