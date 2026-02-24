import { describe, it, expect, beforeEach, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

const runLifecycleFlow = async (findingId: string) => {
  await invoke('count_agent_audit_findings', {
    severityFilter: null,
    statusFilter: null,
    lifecycleStageFilter: null,
    conversationId: null,
    search: null,
  })

  await invoke('list_agent_audit_findings', {
    limit: 10,
    offset: 0,
    severityFilter: null,
    statusFilter: null,
    lifecycleStageFilter: null,
    conversationId: null,
    search: null,
  })

  await invoke('transition_agent_audit_finding_lifecycle', {
    request: {
      finding_id: findingId,
      lifecycle_stage: 'confirmed',
      verification_status: 'passed',
      provenance: {
        source: 'security_center_ui_test',
      },
    },
  })

  await invoke('list_agent_audit_findings', {
    limit: 10,
    offset: 0,
    severityFilter: null,
    statusFilter: null,
    lifecycleStageFilter: null,
    conversationId: null,
    search: null,
  })
}

describe('Code Audit Lifecycle Flow Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should call expected tauri commands in lifecycle transition flow', async () => {
    mockInvoke.mockResolvedValue({ success: true, data: [] })

    await runLifecycleFlow('finding-001')

    expect(mockInvoke).toHaveBeenCalledTimes(4)
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'count_agent_audit_findings', expect.any(Object))
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'list_agent_audit_findings', expect.any(Object))
    expect(mockInvoke).toHaveBeenNthCalledWith(
      3,
      'transition_agent_audit_finding_lifecycle',
      expect.objectContaining({
        request: expect.objectContaining({
          finding_id: 'finding-001',
          lifecycle_stage: 'confirmed',
          verification_status: 'passed',
        }),
      }),
    )
    expect(mockInvoke).toHaveBeenNthCalledWith(4, 'list_agent_audit_findings', expect.any(Object))
  })
})

