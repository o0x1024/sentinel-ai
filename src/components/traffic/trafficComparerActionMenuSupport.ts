import type { TrafficContextMenuActionItem } from './trafficContextMenuSectionSupport'

export type ComparerActionSection = 'copy' | 'baseline' | 'transfer'

export type ComparerActionKey =
  | 'copyLeft'
  | 'copyRight'
  | 'swapSides'
  | 'pinLeftBaseline'
  | 'pinRightBaseline'
  | 'clearPinnedBaseline'
  | 'sendLeftToRepeater'
  | 'sendRightToRepeater'

export interface ComparerActionMenuItem extends TrafficContextMenuActionItem {
  key: ComparerActionKey
  section: ComparerActionSection
  disabled: boolean
}

type BuildComparerActionMenuItemsOptions = {
  actions: Partial<Record<ComparerActionKey, () => void | Promise<void>>>
  visible?: Partial<Record<ComparerActionKey, boolean>>
  enabled?: Partial<Record<ComparerActionKey, boolean>>
}

const COMPARER_ACTION_META: Record<
  ComparerActionKey,
  Pick<ComparerActionMenuItem, 'section' | 'iconClass' | 'labelKey'>
> = {
  copyLeft: {
    section: 'copy',
    iconClass: 'fas fa-copy text-secondary',
    labelKey: 'copyLeft',
  },
  copyRight: {
    section: 'copy',
    iconClass: 'fas fa-copy text-secondary',
    labelKey: 'copyRight',
  },
  swapSides: {
    section: 'copy',
    iconClass: 'fas fa-exchange-alt text-info',
    labelKey: 'swapSides',
  },
  pinLeftBaseline: {
    section: 'baseline',
    iconClass: 'fas fa-thumbtack text-warning',
    labelKey: 'pinLeftBaseline',
  },
  pinRightBaseline: {
    section: 'baseline',
    iconClass: 'fas fa-thumbtack text-warning',
    labelKey: 'pinRightBaseline',
  },
  clearPinnedBaseline: {
    section: 'baseline',
    iconClass: 'fas fa-times-circle text-warning',
    labelKey: 'clearPinnedBaseline',
  },
  sendLeftToRepeater: {
    section: 'transfer',
    iconClass: 'fas fa-redo text-primary',
    labelKey: 'sendLeftToRepeater',
  },
  sendRightToRepeater: {
    section: 'transfer',
    iconClass: 'fas fa-redo text-primary',
    labelKey: 'sendRightToRepeater',
  },
}

const COMPARER_ACTION_ORDER: ComparerActionKey[] = [
  'copyLeft',
  'copyRight',
  'swapSides',
  'pinLeftBaseline',
  'pinRightBaseline',
  'clearPinnedBaseline',
  'sendLeftToRepeater',
  'sendRightToRepeater',
]

export function buildComparerActionMenuItems(
  options: BuildComparerActionMenuItemsOptions,
): ComparerActionMenuItem[] {
  return COMPARER_ACTION_ORDER
    .filter((key) => options.actions[key] && options.visible?.[key] !== false)
    .map((key) => ({
      key,
      ...COMPARER_ACTION_META[key],
      onClick: options.actions[key]!,
      disabled: options.enabled?.[key] === false,
    }))
}
