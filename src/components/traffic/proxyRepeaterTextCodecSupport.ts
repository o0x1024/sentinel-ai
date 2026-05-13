import { nextTick } from 'vue'
import type { RepeaterTab } from './proxyRepeaterTypes'
import { convertRepeaterPrettyRequestToRaw, formatRepeaterPrettyRequest } from './trafficRepeaterPrettyRequestSupport'
import {
  getTrafficTextCodecErrorMessage,
  hasNonEmptyTextSelection,
  replaceTrafficTextSelection,
  transformTrafficTextCodec,
  type TrafficTextCodecAction,
  type TrafficTextSelection,
} from './trafficTextCodecSupport'

export function applyRepeaterRequestTextCodec(
  tab: RepeaterTab,
  selection: TrafficTextSelection,
  action: TrafficTextCodecAction,
) {
  const currentText = tab.requestTab === 'pretty' ? tab.prettyRequest : tab.rawRequest
  const replacement = transformTrafficTextCodec(currentText.slice(selection.from, selection.to), action)
  const next = replaceTrafficTextSelection(currentText, selection, replacement)

  if (tab.requestTab === 'pretty') {
    tab.prettyRequest = next.content
    tab.rawRequest = convertRepeaterPrettyRequestToRaw(next.content)
  } else {
    tab.rawRequest = next.content
    tab.prettyRequest = formatRepeaterPrettyRequest(next.content)
  }

  tab.modified = true
  tab.userEdited = true
  return next
}

export async function runRepeaterRequestTextCodecAction(options: {
  tab: RepeaterTab | null
  selection: TrafficTextSelection | null
  action: TrafficTextCodecAction
  editor: {
    setSelection?: (from: number, to: number) => void
    focus?: () => void
  } | null | undefined
  onReadonly: () => void
  onNoSelection: () => void
  onSuccess: () => void
  onFailure: (message: string) => void
}) {
  if (!options.tab || options.tab.requestTab === 'hex') {
    options.onReadonly()
    return
  }
  if (!hasNonEmptyTextSelection(options.selection)) {
    options.onNoSelection()
    return
  }

  try {
    const next = applyRepeaterRequestTextCodec(options.tab, options.selection, options.action)
    await nextTick()
    options.editor?.setSelection?.(next.selectionStart, next.selectionEnd)
    options.editor?.focus?.()
    options.onSuccess()
  } catch (error) {
    options.onFailure(getTrafficTextCodecErrorMessage(error))
  }
}
