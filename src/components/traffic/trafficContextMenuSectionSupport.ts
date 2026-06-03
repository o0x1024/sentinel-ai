export interface TrafficContextMenuActionItem {
  key: string
  iconClass: string
  labelKey: string
  onClick: () => void | Promise<void>
  disabled?: boolean
}

export interface TrafficContextMenuSection {
  key: string
  items: TrafficContextMenuActionItem[]
}

type BuildTrafficContextMenuSectionsInput = Array<{
  key: string
  items: Array<TrafficContextMenuActionItem | null | undefined | false>
}>

export function buildTrafficContextMenuSections(
  sections: BuildTrafficContextMenuSectionsInput,
): TrafficContextMenuSection[] {
  return sections
    .map((section) => ({
      key: section.key,
      items: section.items.filter((item): item is TrafficContextMenuActionItem => Boolean(item)),
    }))
    .filter((section) => section.items.length > 0)
}
