import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import {
  useShellPermissionHistoryPanel,
  type ShellPermissionHistoryEntry,
} from './useShellPermissionHistoryPanel'

const STORAGE_KEY = 'sentinel:settings:shell-permission-history-panel:v1'

const sampleHistory: ShellPermissionHistoryEntry[] = [
  {
    timestamp: '2026-04-15T08:00:00.000Z',
    session_id: 'session-1',
    execution_id: 'exec-1',
    command: 'git status',
    decision: 'allow',
    allowed: true,
    semantic_kind: 'read_only',
    semantic_code: 'read_only.git_inspection',
    semantic_summary_key: 'tools.shell.semanticSummaries.read_only',
    semantic_reason_key: 'tools.shell.semanticReasons.read_only',
    suggested_allow_rules: [],
    persisted_allow_rules: [],
  },
  {
    timestamp: '2026-04-15T09:00:00.000Z',
    session_id: 'session-2',
    execution_id: 'exec-2',
    command: 'docker run app',
    decision: 'deny',
    allowed: false,
    semantic_kind: 'dangerous',
    semantic_code: 'dangerous.container_runtime',
    semantic_summary_key: 'tools.shell.semanticSummaries.dangerous',
    semantic_reason_key: 'tools.shell.semanticReasons.dangerous',
    suggested_allow_rules: [],
    persisted_allow_rules: [],
  },
  {
    timestamp: '2026-04-15T10:00:00.000Z',
    session_id: 'session-3',
    execution_id: 'exec-3',
    command: 'kubectl apply -f deploy.yaml',
    decision: 'allow_forever',
    allowed: true,
    semantic_kind: 'mutating',
    semantic_code: 'mutating.cluster_change',
    semantic_summary_key: 'tools.shell.semanticSummaries.mutating',
    semantic_reason_key: 'tools.shell.semanticReasons.mutating',
    suggested_allow_rules: [
      {
        rule: 'kubectl apply',
        reason_key: 'tools.shell.allowRuleReasons.command_prefix',
      },
    ],
    persisted_allow_rules: ['kubectl apply'],
  },
]

const flushMountedUpdates = async () => {
  await Promise.resolve()
  await nextTick()
  await Promise.resolve()
}

const mountController = async () => {
  let controller!: ReturnType<typeof useShellPermissionHistoryPanel>
  const Harness = defineComponent({
    setup() {
      controller = useShellPermissionHistoryPanel()
      return () => h('div')
    },
  })

  const wrapper = mount(Harness)
  await flushMountedUpdates()
  return { wrapper, controller }
}

describe('useShellPermissionHistoryPanel', () => {
  beforeEach(() => {
    window.localStorage.clear()
    global.testUtils.mockInvoke.mockReset()
  })

  it('restores persisted filters and uses them for the initial query', async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        executionIdFilter: 'exec-2',
        commandSearchFilter: 'docker',
        selectedDate: '2026-04-15',
        recentDays: 14,
        decisionFilter: 'deny',
        semanticKindFilter: 'dangerous',
        currentLimit: 40,
        expandedEntries: {
          '2026-04-15T09:00:00.000Z-session-2-docker run app': true,
        },
      })
    )
    global.testUtils.mockInvoke.mockResolvedValue(sampleHistory)

    const { wrapper, controller } = await mountController()

    expect(global.testUtils.mockInvoke).toHaveBeenCalledWith('get_shell_permission_history', {
      request: {
        date: '2026-04-15',
        execution_id: 'exec-2',
        limit: 40,
      },
    })
    expect(controller.executionIdFilter.value).toBe('exec-2')
    expect(controller.commandSearchFilter.value).toBe('docker')
    expect(controller.decisionFilter.value).toBe('deny')
    expect(controller.semanticKindFilter.value).toBe('dangerous')
    expect(controller.isExpanded(sampleHistory[1])).toBe(true)

    wrapper.unmount()
  })

  it('filters visible history locally by decision, semantic kind, and command text', async () => {
    global.testUtils.mockInvoke.mockResolvedValue(sampleHistory)

    const { wrapper, controller } = await mountController()

    expect(controller.visibleHistory.value).toHaveLength(3)

    controller.decisionFilter.value = 'deny'
    controller.semanticKindFilter.value = 'dangerous'
    controller.commandSearchFilter.value = 'docker'
    await nextTick()

    expect(controller.history.value).toHaveLength(3)
    expect(controller.visibleHistory.value).toHaveLength(1)
    expect(controller.visibleHistory.value[0]?.command).toBe('docker run app')

    wrapper.unmount()
  })

  it('loads more history by increasing the requested limit', async () => {
    global.testUtils.mockInvoke
      .mockResolvedValueOnce(sampleHistory.slice(0, 2))
      .mockResolvedValueOnce([
        ...sampleHistory,
        ...Array.from({ length: 37 }, (_, index) => ({
          ...sampleHistory[0],
          timestamp: `2026-04-16T00:${String(index).padStart(2, '0')}:00.000Z`,
          session_id: `session-extra-${index}`,
          command: `git status ${index}`,
        })),
      ])

    const { wrapper, controller } = await mountController()

    expect(global.testUtils.mockInvoke).toHaveBeenNthCalledWith(1, 'get_shell_permission_history', {
      request: {
        days: 7,
        execution_id: null,
        limit: 20,
      },
    })

    controller.loadMoreHistory()
    await flushMountedUpdates()

    expect(global.testUtils.mockInvoke).toHaveBeenNthCalledWith(2, 'get_shell_permission_history', {
      request: {
        days: 7,
        execution_id: null,
        limit: 40,
      },
    })
    expect(controller.hasMoreHistory.value).toBe(true)

    wrapper.unmount()
  })

  it('persists expanded entry state to localStorage', async () => {
    global.testUtils.mockInvoke.mockResolvedValue(sampleHistory)

    const { wrapper, controller } = await mountController()

    controller.toggleExpandedAndPersist(sampleHistory[2])
    await nextTick()

    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) || '{}')
    expect(stored.expandedEntries).toMatchObject({
      [controller.entryKey(sampleHistory[2])]: true,
    })

    wrapper.unmount()
  })
})
