import { computed, ref } from 'vue'

export interface PacketCaptureStatusSnapshot {
  isCapturing: boolean
  selectedInterface: string
  selectedInterfaceLabel: string
  packetCount: number
  filteredPacketCount: number
}

const isCapturing = ref(false)
const selectedInterface = ref('')
const selectedInterfaceLabel = ref('')
const packetCount = ref(0)
const filteredPacketCount = ref(0)

export function usePacketCaptureStatusStore() {
  function setStatus(next: Partial<PacketCaptureStatusSnapshot>) {
    if (typeof next.isCapturing === 'boolean') isCapturing.value = next.isCapturing
    if (typeof next.selectedInterface === 'string') selectedInterface.value = next.selectedInterface
    if (typeof next.selectedInterfaceLabel === 'string') selectedInterfaceLabel.value = next.selectedInterfaceLabel
    if (typeof next.packetCount === 'number') packetCount.value = next.packetCount
    if (typeof next.filteredPacketCount === 'number') filteredPacketCount.value = next.filteredPacketCount
  }

  function reset() {
    isCapturing.value = false
    selectedInterface.value = ''
    selectedInterfaceLabel.value = ''
    packetCount.value = 0
    filteredPacketCount.value = 0
  }

  const snapshot = computed<PacketCaptureStatusSnapshot>(() => ({
    isCapturing: isCapturing.value,
    selectedInterface: selectedInterface.value,
    selectedInterfaceLabel: selectedInterfaceLabel.value,
    packetCount: packetCount.value,
    filteredPacketCount: filteredPacketCount.value,
  }))

  return {
    isCapturing,
    selectedInterface,
    selectedInterfaceLabel,
    packetCount,
    filteredPacketCount,
    snapshot,
    setStatus,
    reset,
  }
}
