import { onBeforeUnmount, ref } from 'vue'

export function useImmersiveAnnouncement() {
  const announcement = ref('')
  let frameId = 0

  const clearScheduledAnnouncement = () => {
    if (typeof window === 'undefined' || frameId === 0) {
      return
    }

    window.cancelAnimationFrame(frameId)
    frameId = 0
  }

  const announce = (message: string) => {
    clearScheduledAnnouncement()
    announcement.value = ''

    if (typeof window === 'undefined') {
      announcement.value = message
      return
    }

    frameId = window.requestAnimationFrame(() => {
      announcement.value = message
      frameId = 0
    })
  }

  onBeforeUnmount(() => {
    clearScheduledAnnouncement()
  })

  return {
    announcement,
    announce,
  }
}
