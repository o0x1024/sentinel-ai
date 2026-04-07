import {
  buildTrafficContextMenuSections,
  type TrafficContextMenuActionItem,
  type TrafficContextMenuSection,
} from './trafficContextMenuSectionSupport'

type TrafficRequestContextMenuItems = Array<TrafficContextMenuActionItem | null | undefined | false>

type BuildTrafficRequestContextMenuSectionsOptions = {
  sendItems?: TrafficRequestContextMenuItems
  compareItems?: TrafficRequestContextMenuItems
  requestItems?: TrafficRequestContextMenuItems
  assistantItems?: TrafficRequestContextMenuItems
  editorItems?: TrafficRequestContextMenuItems
}

export function buildTrafficRequestContextMenuSections(
  options: BuildTrafficRequestContextMenuSectionsOptions,
): TrafficContextMenuSection[] {
  return buildTrafficContextMenuSections([
    {
      key: 'send',
      items: options.sendItems ?? [],
    },
    {
      key: 'compare',
      items: options.compareItems ?? [],
    },
    {
      key: 'request',
      items: options.requestItems ?? [],
    },
    {
      key: 'assistant',
      items: options.assistantItems ?? [],
    },
    {
      key: 'editor',
      items: options.editorItems ?? [],
    },
  ])
}
