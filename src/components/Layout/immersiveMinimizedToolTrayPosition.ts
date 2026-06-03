export const IMMERSIVE_MINIMIZED_TOOL_TRAY_POSITION_STORAGE_KEY =
  'sentinel:immersive-minimized-tool-tray-position:v1'

export const IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN = 16
export const IMMERSIVE_MINIMIZED_TOOL_TRAY_SNAP_THRESHOLD = 24
export const IMMERSIVE_MINIMIZED_TOOL_TRAY_MAGNETIC_THRESHOLD = 88

export interface ImmersiveTrayPosition {
  x: number
  y: number
}

export interface ImmersiveTrayViewport {
  width: number
  height: number
}

export interface ImmersiveTraySize {
  width: number
  height: number
}

export interface ImmersiveTrayRect extends ImmersiveTrayPosition, ImmersiveTraySize {}

export interface ImmersiveTraySnapEdges {
  left: boolean
  right: boolean
  top: boolean
  bottom: boolean
}

export type ImmersiveTrayDockSide = 'left' | 'right' | 'top' | 'bottom' | null

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

export function readImmersiveMinimizedToolTrayPosition() {
  if (typeof window === 'undefined') {
    return null
  }

  try {
    const raw = window.localStorage.getItem(IMMERSIVE_MINIMIZED_TOOL_TRAY_POSITION_STORAGE_KEY)
    if (!raw) {
      return null
    }

    const parsed = JSON.parse(raw) as Partial<ImmersiveTrayPosition> | null
    if (!parsed || !isFiniteNumber(parsed.x) || !isFiniteNumber(parsed.y)) {
      return null
    }

    return {
      x: parsed.x,
      y: parsed.y,
    }
  } catch {
    return null
  }
}

export function persistImmersiveMinimizedToolTrayPosition(position: ImmersiveTrayPosition) {
  if (typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(
    IMMERSIVE_MINIMIZED_TOOL_TRAY_POSITION_STORAGE_KEY,
    JSON.stringify(position),
  )
}

export function buildDefaultImmersiveMinimizedToolTrayPosition(
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
) {
  return clampImmersiveMinimizedToolTrayPosition(
    {
      x: viewport.width - traySize.width - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
      y: viewport.height - traySize.height - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
    },
    viewport,
    traySize,
  )
}

export function clampImmersiveMinimizedToolTrayPosition(
  position: ImmersiveTrayPosition,
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
) {
  const maxX = Math.max(
    IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
    viewport.width - traySize.width - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
  )
  const maxY = Math.max(
    IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
    viewport.height - traySize.height - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
  )

  return {
    x: Math.min(Math.max(IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN, position.x), maxX),
    y: Math.min(Math.max(IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN, position.y), maxY),
  }
}

export function resolveImmersiveMinimizedToolTraySnapEdges(
  position: ImmersiveTrayPosition,
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
  threshold = IMMERSIVE_MINIMIZED_TOOL_TRAY_SNAP_THRESHOLD,
): ImmersiveTraySnapEdges {
  const clamped = clampImmersiveMinimizedToolTrayPosition(position, viewport, traySize)
  const leftDistance = clamped.x - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const rightDistance =
    viewport.width - (clamped.x + traySize.width) - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const topDistance = clamped.y - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const bottomDistance =
    viewport.height - (clamped.y + traySize.height) - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN

  return {
    left: leftDistance <= threshold && leftDistance <= rightDistance,
    right: rightDistance <= threshold && rightDistance < leftDistance,
    top: topDistance <= threshold && topDistance <= bottomDistance,
    bottom: bottomDistance <= threshold && bottomDistance < topDistance,
  }
}

export function snapImmersiveMinimizedToolTrayPosition(
  position: ImmersiveTrayPosition,
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
  threshold = IMMERSIVE_MINIMIZED_TOOL_TRAY_SNAP_THRESHOLD,
) {
  const clamped = clampImmersiveMinimizedToolTrayPosition(position, viewport, traySize)
  const snapEdges = resolveImmersiveMinimizedToolTraySnapEdges(
    clamped,
    viewport,
    traySize,
    threshold,
  )

  return {
    x: snapEdges.left
      ? IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
      : snapEdges.right
        ? viewport.width - traySize.width - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
        : clamped.x,
    y: snapEdges.top
      ? IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
      : snapEdges.bottom
        ? viewport.height - traySize.height - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
        : clamped.y,
  }
}

export function applyImmersiveMinimizedToolTrayMagneticAttraction(
  position: ImmersiveTrayPosition,
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
  threshold = IMMERSIVE_MINIMIZED_TOOL_TRAY_MAGNETIC_THRESHOLD,
) {
  const clamped = clampImmersiveMinimizedToolTrayPosition(position, viewport, traySize)
  const leftSnapX = IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const rightSnapX = viewport.width - traySize.width - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const topSnapY = IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const bottomSnapY = viewport.height - traySize.height - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN

  const applyAxisMagnetism = (current: number, firstTarget: number, secondTarget: number) => {
    const firstDistance = Math.abs(firstTarget - current)
    const secondDistance = Math.abs(secondTarget - current)
    const nearestTarget = firstDistance <= secondDistance ? firstTarget : secondTarget
    const nearestDistance = Math.min(firstDistance, secondDistance)

    if (nearestDistance > threshold) {
      return current
    }

    const pullRatio = 1 - nearestDistance / threshold
    const easedPull = pullRatio * pullRatio * 0.35
    return current + (nearestTarget - current) * easedPull
  }

  return {
    x: applyAxisMagnetism(clamped.x, leftSnapX, rightSnapX),
    y: applyAxisMagnetism(clamped.y, topSnapY, bottomSnapY),
  }
}

export function resolveImmersiveMinimizedToolTrayDockSide(
  position: ImmersiveTrayPosition,
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
  threshold = IMMERSIVE_MINIMIZED_TOOL_TRAY_SNAP_THRESHOLD,
): ImmersiveTrayDockSide {
  const snapEdges = resolveImmersiveMinimizedToolTraySnapEdges(
    position,
    viewport,
    traySize,
    threshold,
  )

  if (snapEdges.left) {
    return 'left'
  }

  if (snapEdges.right) {
    return 'right'
  }

  if (snapEdges.top) {
    return 'top'
  }

  if (snapEdges.bottom) {
    return 'bottom'
  }

  return null
}

export function doImmersiveTrayRectsOverlap(
  first: ImmersiveTrayRect,
  second: ImmersiveTrayRect,
) {
  return !(
    first.x + first.width <= second.x
    || second.x + second.width <= first.x
    || first.y + first.height <= second.y
    || second.y + second.height <= first.y
  )
}

export function resolveImmersiveMinimizedToolTrayPosition(
  preferredPosition: ImmersiveTrayPosition,
  viewport: ImmersiveTrayViewport,
  traySize: ImmersiveTraySize,
  dockRect?: ImmersiveTrayRect | null,
) {
  const preferred = clampImmersiveMinimizedToolTrayPosition(preferredPosition, viewport, traySize)
  if (!dockRect) {
    return preferred
  }

  const currentRect = {
    ...preferred,
    ...traySize,
  }

  if (!doImmersiveTrayRectsOverlap(currentRect, dockRect)) {
    return preferred
  }

  const candidates: ImmersiveTrayPosition[] = [
    {
      x: dockRect.x - traySize.width - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
      y: preferred.y,
    },
    {
      x: dockRect.x + dockRect.width + IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
      y: preferred.y,
    },
    {
      x: preferred.x,
      y: dockRect.y - traySize.height - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
    },
    {
      x: preferred.x,
      y: dockRect.y + dockRect.height + IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
    },
    buildDefaultImmersiveMinimizedToolTrayPosition(viewport, traySize),
  ]

  for (const candidate of candidates) {
    const next = clampImmersiveMinimizedToolTrayPosition(candidate, viewport, traySize)
    if (
      !doImmersiveTrayRectsOverlap(
        {
          ...next,
          ...traySize,
        },
        dockRect,
      )
    ) {
      return next
    }
  }

  return preferred
}
