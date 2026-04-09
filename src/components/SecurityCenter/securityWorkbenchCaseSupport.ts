import { invoke } from '@tauri-apps/api/core'
import type {
  CommandResponse as SettingsCommandResponse,
  SystemAgentProfilePayload,
} from '@/components/Settings/systemAgentSettingsSupport'
import type {
  WorkbenchCase,
  WorkbenchCaseDetailResult,
  WorkbenchExecutionDraft,
  WorkbenchExecutionDraftStatus,
  WorkbenchExecutionRun,
  WorkbenchCaseListQuery,
  WorkbenchCaseListResult,
  WorkbenchCasePatch,
  WorkbenchDeleteCasesResult,
  WorkbenchFindingSyncResult,
  WorkbenchNote,
  WorkbenchNoteKind,
  WorkbenchReplayPlan,
} from './securityWorkbenchTypes'

interface CommandResponse<T> {
  success: boolean
  data?: T | null
  error?: string | null
}

const unwrapResponse = <T>(response: CommandResponse<T>, fallbackMessage: string): T => {
  if (response.success && response.data != null) {
    return response.data
  }

  throw new Error(response.error || fallbackMessage)
}

export const listWorkbenchCases = async (
  query: WorkbenchCaseListQuery = {},
): Promise<WorkbenchCaseListResult> => {
  const response = await invoke<CommandResponse<WorkbenchCaseListResult>>(
    'security_workbench_list_cases',
    {
      request: {
        search: query.search || null,
        status: query.status || null,
        page: query.page ?? 1,
        pageSize: query.pageSize ?? 20,
      },
    },
  )
  return unwrapResponse(response, '加载案件列表失败')
}

export const getWorkbenchCaseDetail = async (
  caseId: string,
): Promise<WorkbenchCaseDetailResult | null> => {
  const response = await invoke<CommandResponse<WorkbenchCaseDetailResult | null>>(
    'security_workbench_get_case_detail',
    {
      request: { caseId },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '加载案件详情失败')
}

export const getOrCreateWorkbenchCaseForFinding = async (
  findingId: string,
): Promise<WorkbenchCase> => {
  const response = await invoke<CommandResponse<WorkbenchCase>>(
    'security_workbench_get_or_create_case_for_finding',
    {
      request: { findingId },
    },
  )
  return unwrapResponse(response, '创建或打开案件失败')
}

export const updateWorkbenchCase = async (
  caseId: string,
  patch: WorkbenchCasePatch,
): Promise<WorkbenchCase | null> => {
  const response = await invoke<CommandResponse<WorkbenchCase | null>>(
    'security_workbench_update_case',
    {
      request: {
        caseId,
        patch: {
          status: patch.status ?? null,
          currentConclusion: patch.currentConclusion ?? null,
          priority: patch.priority ?? null,
          baselineEvidenceId:
            patch.baselineEvidenceId === undefined ? undefined : patch.baselineEvidenceId,
        },
      },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '更新案件失败')
}

export const deleteWorkbenchCases = async (
  caseIds: string[],
): Promise<WorkbenchDeleteCasesResult> => {
  const response = await invoke<CommandResponse<WorkbenchDeleteCasesResult>>(
    'security_workbench_delete_cases',
    {
      request: {
        caseIds,
      },
    },
  )

  return unwrapResponse(response, '删除案件失败')
}

export const addWorkbenchNote = async (
  caseId: string,
  kind: WorkbenchNoteKind,
  body: string,
): Promise<WorkbenchNote | null> => {
  const response = await invoke<CommandResponse<WorkbenchNote | null>>(
    'security_workbench_add_note',
    {
      request: {
        caseId,
        kind,
        body,
        author: '当前用户',
      },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '新增复盘备注失败')
}

export const syncWorkbenchCaseToFinding = async (
  caseId: string,
  options: {
    applySuggestionToCase?: boolean
  } = {},
): Promise<WorkbenchFindingSyncResult | null> => {
  const response = await invoke<CommandResponse<WorkbenchFindingSyncResult | null>>(
    'security_workbench_sync_case_to_finding',
    {
      request: {
        caseId,
        applySuggestionToCase: options.applySuggestionToCase ?? false,
      },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '回写 finding 失败')
}

export const createWorkbenchExecutionDraft = async (
  caseId: string,
  plan: WorkbenchReplayPlan,
): Promise<WorkbenchExecutionDraft | null> => {
  const response = await invoke<CommandResponse<WorkbenchExecutionDraft | null>>(
    'security_workbench_create_execution_draft',
    {
      request: {
        caseId,
        plan,
      },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '创建执行草案失败')
}

export const updateWorkbenchExecutionDraftStatus = async (
  draftId: string,
  status: WorkbenchExecutionDraftStatus,
): Promise<WorkbenchExecutionDraft | null> => {
  const response = await invoke<CommandResponse<WorkbenchExecutionDraft | null>>(
    'security_workbench_update_execution_draft',
    {
      request: {
        draftId,
        status,
      },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '更新执行草案状态失败')
}

export const executeWorkbenchExecutionDraft = async (
  draftId: string,
  options: {
    confirmNonReadonly?: boolean
  } = {},
): Promise<WorkbenchExecutionRun | null> => {
  const response = await invoke<CommandResponse<WorkbenchExecutionRun | null>>(
    'security_workbench_execute_execution_draft',
    {
      request: {
        draftId,
        confirmNonReadonly: options.confirmNonReadonly ?? false,
      },
    },
  )

  if (response.success) {
    return response.data ?? null
  }

  throw new Error(response.error || '执行只读草案失败')
}

export const getWorkbenchAutoModeEnabled = async (): Promise<boolean> => {
  const response = await invoke<SettingsCommandResponse<SystemAgentProfilePayload | null>>(
    'get_system_agent_profile',
    {
      id: 'traffic_active_verifier',
    },
  )

  if (!response.success) {
    throw new Error(response.error || '加载自动验证配置失败')
  }

  return response.data?.safetyPolicy?.autoMode === true
}
