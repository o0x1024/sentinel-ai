export type ImmersiveFloatingDockSide = 'left' | 'right' | 'top' | 'bottom' | null

export interface ImmersiveFloatingPosition {
  x: number
  y: number
}

export function describeImmersiveFloatingDockSide(side: ImmersiveFloatingDockSide) {
  if (side === 'left') {
    return '当前停靠在左侧边缘'
  }

  if (side === 'right') {
    return '当前停靠在右侧边缘'
  }

  if (side === 'top') {
    return '当前停靠在顶部边缘'
  }

  if (side === 'bottom') {
    return '当前停靠在底部边缘'
  }

  return '当前处于浮动位置'
}

export function buildImmersiveFloatingPositionAnnouncement(
  subject: string,
  side: ImmersiveFloatingDockSide,
  position: ImmersiveFloatingPosition,
) {
  if (side === 'left') {
    return `${subject}已停靠到左侧边缘。`
  }

  if (side === 'right') {
    return `${subject}已停靠到右侧边缘。`
  }

  if (side === 'top') {
    return `${subject}已停靠到顶部边缘。`
  }

  if (side === 'bottom') {
    return `${subject}已停靠到底部边缘。`
  }

  return `${subject}已移动到距左 ${Math.round(position.x)} 像素，距上 ${Math.round(position.y)} 像素。`
}
