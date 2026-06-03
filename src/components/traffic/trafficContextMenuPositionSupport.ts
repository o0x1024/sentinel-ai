export function resolveTrafficContextMenuPosition(
  event: MouseEvent,
  menuSize: { width: number; height: number },
) {
  let x = event.clientX
  let y = event.clientY

  if (x + menuSize.width > window.innerWidth) {
    x = window.innerWidth - menuSize.width - 10
  }
  if (y + menuSize.height > window.innerHeight) {
    y = window.innerHeight - menuSize.height - 10
  }

  return {
    x: Math.max(0, x),
    y: Math.max(0, y),
  }
}
