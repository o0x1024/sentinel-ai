import { computed, ref } from 'vue'
import type {
  TrafficWorkbenchSource,
  TrafficWorkbenchToolSession,
} from './trafficWorkbenchTypes'

type WorkbenchTool = TrafficWorkbenchToolSession['tool']

const createDefaultSessions = (): Record<WorkbenchTool, TrafficWorkbenchToolSession> => ({
  repeater: {
    tool: 'repeater',
    title: '重放器',
    source: null,
    updatedAt: 0,
    count: 0,
  },
  intruder: {
    tool: 'intruder',
    title: '爆破器',
    source: null,
    updatedAt: 0,
    count: 0,
  },
  comparer: {
    tool: 'comparer',
    title: '对比器',
    source: null,
    updatedAt: 0,
    count: 0,
  },
  oast: {
    tool: 'oast',
    title: 'OAST',
    source: null,
    updatedAt: 0,
    count: 0,
  },
})

export const useTrafficWorkbenchSessions = () => {
  const sessions = ref(createDefaultSessions())

  function markSession(tool: WorkbenchTool, source: TrafficWorkbenchSource | null) {
    sessions.value = {
      ...sessions.value,
      [tool]: {
        ...sessions.value[tool],
        source,
        updatedAt: Date.now(),
        count: sessions.value[tool].count + 1,
      },
    }
  }

  const sessionList = computed(() =>
    (Object.values(sessions.value) as TrafficWorkbenchToolSession[]).sort(
      (left, right) => right.updatedAt - left.updatedAt,
    ),
  )

  return {
    sessions,
    sessionList,
    markSession,
  }
}
