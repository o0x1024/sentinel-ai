import type { TrafficContextMenuActionItem } from './trafficContextMenuSectionSupport'
import type { TrafficSendTarget } from './trafficSendTargets'

export interface TrafficRequestSendMenuItem extends TrafficContextMenuActionItem {
  key: TrafficSendTarget
}

type BuildTrafficRequestSendMenuItemsOptions = {
  enabledTargets: Record<TrafficSendTarget, boolean>
  supportedTargets: TrafficSendTarget[]
  actions: Partial<Record<TrafficSendTarget, () => void | Promise<void>>>
}

const TARGET_MENU_META: Record<TrafficSendTarget, Pick<TrafficRequestSendMenuItem, 'iconClass' | 'labelKey'>> = {
  draft: {
    iconClass: 'fas fa-redo text-primary',
    labelKey: 'createDraft',
  },
  compare: {
    iconClass: 'fas fa-not-equal text-accent',
    labelKey: 'openCompare',
  },
  attackWorkspace: {
    iconClass: 'fas fa-crosshairs text-secondary',
    labelKey: 'createAttackWorkspace',
  },
}

const TARGET_ORDER: TrafficSendTarget[] = ['draft', 'compare', 'attackWorkspace']

export function buildTrafficRequestSendMenuItems(
  options: BuildTrafficRequestSendMenuItemsOptions,
): TrafficRequestSendMenuItem[] {
  const supportedTargets = new Set(options.supportedTargets)

  return TARGET_ORDER
    .filter((target) => supportedTargets.has(target) && options.enabledTargets[target] && options.actions[target])
    .map((target) => ({
      key: target,
      ...TARGET_MENU_META[target],
      onClick: options.actions[target]!,
    }))
}
