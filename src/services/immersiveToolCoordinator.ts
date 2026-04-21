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

export function closeOtherImmersiveTools(activeId: ImmersiveToolCoordinatorId) {
  immersiveToolEntries.forEach((entry, id) => {
    if (id === activeId) {
      return
    }

    if (!entry.isBlocking()) {
      return
    }

    entry.close()
  })
}

export function closeAllImmersiveTools() {
  immersiveToolEntries.forEach(entry => {
    entry.close()
  })
}
