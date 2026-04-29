import { ref } from 'vue'
import { listTrafficOastRecords } from '@/api/trafficOast'
import type { TrafficOastRecord } from './proxyConfigurationTypes'

const oastRecordCount = ref(0)

export function countValidTrafficOastRecords(records: TrafficOastRecord[]) {
  return records.filter(record => record.hitCount > 0).length
}

export function setTrafficOastRecordCountFromRecords(records: TrafficOastRecord[]) {
  oastRecordCount.value = countValidTrafficOastRecords(records)
}

export async function refreshTrafficOastRecordCount() {
  setTrafficOastRecordCountFromRecords(await listTrafficOastRecords())
}

export function resetTrafficOastRecordCount() {
  oastRecordCount.value = 0
}

export function useTrafficOastRecordCount() {
  return {
    oastRecordCount,
  }
}
