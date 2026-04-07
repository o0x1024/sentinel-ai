import type { TrafficContextMenuActionItem } from './trafficContextMenuSectionSupport'

export interface TrafficContextSubmenuItem extends TrafficContextMenuActionItem {
  suffixText?: string
}

export interface TrafficContextSubmenu {
  key: string
  triggerLabelKey: string
  triggerIconClass: string
  items: TrafficContextSubmenuItem[]
  footerItems?: TrafficContextSubmenuItem[]
  submenuClass?: string
}

type BuildTrafficContextSubmenuOptions = {
  key: string
  triggerLabelKey: string
  triggerIconClass: string
  items: Array<TrafficContextSubmenuItem | null | undefined | false>
  footerItems?: Array<TrafficContextSubmenuItem | null | undefined | false>
  submenuClass?: string
}

export function buildTrafficContextSubmenu(
  options: BuildTrafficContextSubmenuOptions,
): TrafficContextSubmenu | null {
  const items = options.items.filter((item): item is TrafficContextSubmenuItem => Boolean(item))
  const footerItems = options.footerItems?.filter((item): item is TrafficContextSubmenuItem => Boolean(item)) ?? []

  if (!items.length && !footerItems.length) {
    return null
  }

  return {
    key: options.key,
    triggerLabelKey: options.triggerLabelKey,
    triggerIconClass: options.triggerIconClass,
    items,
    footerItems: footerItems.length ? footerItems : undefined,
    submenuClass: options.submenuClass,
  }
}
