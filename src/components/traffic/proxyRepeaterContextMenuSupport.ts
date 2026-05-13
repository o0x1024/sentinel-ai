import { resolveTrafficContextMenuPosition } from './trafficContextMenuPositionSupport'

export type RepeaterContextMenuPane = 'request' | 'response'

const REPEATER_CONTEXT_MENU_SIZE = { width: 220, height: 400 }
const REPEATER_TAB_CONTEXT_MENU_SIZE = { width: 180, height: 136 }

export function buildRepeaterContextMenuState(
  event: MouseEvent,
  pane: RepeaterContextMenuPane,
  selection: { from: number; to: number } | null,
) {
  const position = resolveTrafficContextMenuPosition(event, REPEATER_CONTEXT_MENU_SIZE)
  return {
    visible: true,
    x: position.x,
    y: position.y,
    width: REPEATER_CONTEXT_MENU_SIZE.width,
    height: REPEATER_CONTEXT_MENU_SIZE.height,
    pane,
    selection,
  }
}

export function buildRepeaterTabContextMenuState(event: MouseEvent, tabIndex: number) {
  const position = resolveTrafficContextMenuPosition(event, REPEATER_TAB_CONTEXT_MENU_SIZE)
  return {
    visible: true,
    x: position.x,
    y: position.y,
    tabIndex,
  }
}
