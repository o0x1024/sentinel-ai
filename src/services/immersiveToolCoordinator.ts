export type ImmersiveToolCoordinatorId =
  | 'traffic-assistant'
  | 'traffic-workbench'
  | 'traffic-control'
  | 'traffic-basket'
  | 'traffic-settings'
  | 'security-center'

type ImmersiveToolCloser = () => void
type ImmersiveToolBlockingStateReader = () => boolean

const immersiveToolEntries = new Map<
  ImmersiveToolCoordinatorId,
  {
    close: ImmersiveToolCloser
    isBlocking: ImmersiveToolBlockingStateReader
  }
>()

const immersiveToolLayerOrder: ImmersiveToolCoordinatorId[] = [
  'traffic-settings',
  'security-center',
  'traffic-basket',
  'traffic-assistant',
  'traffic-control',
  'traffic-workbench',
]

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

export function closeAllImmersiveTools() {
  immersiveToolEntries.forEach(entry => {
    entry.close()
  })
}

export function closeTopmostImmersiveTool() {
  for (const id of immersiveToolLayerOrder) {
    const entry = immersiveToolEntries.get(id)
    if (!entry || !entry.isBlocking()) {
      continue
    }

    entry.close()
    return true
  }

  return false
}

export function hasOpenImmersiveTools() {
  for (const entry of immersiveToolEntries.values()) {
    if (entry.isBlocking()) {
      return true
    }
  }

  return false
}
