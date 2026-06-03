import { invoke } from '@tauri-apps/api/core'
import type { IntruderWorkspace } from '@/components/traffic/intruder/workspaceSupport'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export interface PersistedIntruderWorkspaceSessionStore {
  activeWorkspaceId: string | null
  workspaces: Array<Partial<IntruderWorkspace>>
}

async function expectCommandData<T>(command: string, response: CommandResponse<T>) {
  if (!response.success) {
    throw new Error(response.error || `Failed to execute ${command}`)
  }
  if (response.data === undefined) {
    throw new Error(`Missing response data for ${command}`)
  }
  return response.data
}

export async function loadIntruderWorkspaceSessionStore(): Promise<PersistedIntruderWorkspaceSessionStore> {
  const response = await invoke<CommandResponse<PersistedIntruderWorkspaceSessionStore>>(
    'load_intruder_workspace_session_store',
  )
  return expectCommandData('load_intruder_workspace_session_store', response)
}

export async function saveIntruderWorkspaceSessionStore(
  store: PersistedIntruderWorkspaceSessionStore,
): Promise<void> {
  const response = await invoke<CommandResponse<null>>(
    'save_intruder_workspace_session_store',
    { store },
  )
  await expectCommandData('save_intruder_workspace_session_store', {
    ...response,
    data: response.data ?? null,
  })
}
