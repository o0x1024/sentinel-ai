import type { TrafficContextMenuActionItem } from './trafficContextMenuSectionSupport'

export type TrafficRequestAction = 'copyUrl' | 'copyRequest' | 'copyAsCurl' | 'openInBrowser'

export interface TrafficRequestActionMenuItem extends TrafficContextMenuActionItem {
  key: TrafficRequestAction
}

type BuildTrafficRequestActionMenuItemsOptions = {
  supportedActions: TrafficRequestAction[]
  actions: Partial<Record<TrafficRequestAction, () => void | Promise<void>>>
  labelKeyOverrides?: Partial<Record<TrafficRequestAction, string>>
}

const ACTION_MENU_META: Record<TrafficRequestAction, Pick<TrafficRequestActionMenuItem, 'iconClass' | 'labelKey'>> = {
  copyUrl: {
    iconClass: 'fas fa-link text-info',
    labelKey: 'copyUrl',
  },
  copyRequest: {
    iconClass: 'fas fa-copy text-secondary',
    labelKey: 'copyRequest',
  },
  copyAsCurl: {
    iconClass: 'fas fa-terminal text-warning',
    labelKey: 'copyAsCurl',
  },
  openInBrowser: {
    iconClass: 'fas fa-globe text-success',
    labelKey: 'openInBrowser',
  },
}

const ACTION_ORDER: TrafficRequestAction[] = ['copyUrl', 'copyRequest', 'copyAsCurl', 'openInBrowser']

export function buildTrafficRequestActionMenuItems(
  options: BuildTrafficRequestActionMenuItemsOptions,
): TrafficRequestActionMenuItem[] {
  const supportedActions = new Set(options.supportedActions)

  return ACTION_ORDER
    .filter((action) => supportedActions.has(action) && options.actions[action])
    .map((action) => ({
      key: action,
      iconClass: ACTION_MENU_META[action].iconClass,
      labelKey: options.labelKeyOverrides?.[action] ?? ACTION_MENU_META[action].labelKey,
      onClick: options.actions[action]!,
    }))
}
