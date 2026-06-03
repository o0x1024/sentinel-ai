export type ImmersiveToolCoordinatorId =
  | 'traffic-assistant'
  | 'traffic-workbench'
  | 'traffic-control'
  | 'traffic-basket'
  | 'traffic-plugins'
  | 'traffic-settings'
  | 'security-center'

type ImmersiveToolCloser = () => void
type ImmersiveToolBlockingStateReader = () => boolean
export type ImmersiveToolCloseScope = 'all' | 'transient'

const immersiveToolEntries = new Map<
  ImmersiveToolCoordinatorId,
  {
    close: ImmersiveToolCloser
    isBlocking: ImmersiveToolBlockingStateReader
  }
>()

const immersiveToolLayerOrder: ImmersiveToolCoordinatorId[] = [
  'traffic-plugins',
  'traffic-settings',
  'security-center',
  'traffic-basket',
  'traffic-assistant',
  'traffic-control',
  'traffic-workbench',
]

const persistentImmersiveToolIds = new Set<ImmersiveToolCoordinatorId>([
  'traffic-workbench',
])

function canCloseInScope(id: ImmersiveToolCoordinatorId, scope: ImmersiveToolCloseScope) {
  return scope === 'all' || !persistentImmersiveToolIds.has(id)
}

export function registerImmersiveToolCloser(
  id: ImmersiveToolCoordinatorId,
  closer: ImmersiveToolCloser,
  isBlocking: ImmersiveToolBlockingStateReader = () => true,
) {
  immersiveToolEntries.set(id, {
    close: closer,
    isBlocking,
  })
}

export function closeAllImmersiveTools(scope: ImmersiveToolCloseScope = 'all') {
  immersiveToolEntries.forEach((entry, id) => {
    if (!canCloseInScope(id, scope)) {
      return
    }

    entry.close()
  })
}

export function closeTopmostImmersiveTool(scope: ImmersiveToolCloseScope = 'all') {
  for (const id of immersiveToolLayerOrder) {
    if (!canCloseInScope(id, scope)) {
      continue
    }

    const entry = immersiveToolEntries.get(id)
    if (!entry || !entry.isBlocking()) {
      continue
    }

    entry.close()
    return true
  }

  return false
}

export function hasOpenImmersiveTools(scope: ImmersiveToolCloseScope = 'all') {
  for (const [id, entry] of immersiveToolEntries.entries()) {
    if (!canCloseInScope(id, scope)) {
      continue
    }

    if (entry.isBlocking()) {
      return true
    }
  }

  return false
}
