import { invoke } from '@tauri-apps/api/core'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export type BrowserShellWriteStatus =
  | 'pending_approval'
  | 'queued'
  | 'dispatching'
  | 'delivered'
  | 'failed'
  | 'rejected'

export interface BrowserShellSession {
  id: string
  tabId: number | null
  frameId: number | null
  pageUrl: string
  pageTitle: string | null
  wsUrl: string
  protocol: string | null
  terminalKind: string
  writable: boolean
  connected: boolean
  lastSeenAt: string
}

export interface BrowserShellFrame {
  id: string
  sessionId: string
  direction: string
  frameType: string
  textPreview: string | null
  payloadBase64: string | null
  receivedAt: string
}

export interface BrowserShellFrameList {
  sessionId: string
  frames: BrowserShellFrame[]
}

export interface BrowserShellWriteRequest {
  requestId: string
  sessionId: string
  inputText: string
  inputBase64: string
  requiresApproval: boolean
  status: BrowserShellWriteStatus
  error: string | null
  createdAt: string
  updatedAt: string
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

export async function listBrowserShellSessions(): Promise<BrowserShellSession[]> {
  const response = await invoke<CommandResponse<BrowserShellSession[]>>('list_traffic_browser_shell_sessions')
  return expectCommandData('list_traffic_browser_shell_sessions', response)
}

export async function getBrowserShellFrames(
  sessionId: string,
  limit = 80,
): Promise<BrowserShellFrameList> {
  const response = await invoke<CommandResponse<BrowserShellFrameList>>('get_traffic_browser_shell_frames', {
    sessionId,
    limit,
  })
  return expectCommandData('get_traffic_browser_shell_frames', response)
}

export async function listBrowserShellWriteRequests(): Promise<BrowserShellWriteRequest[]> {
  const response = await invoke<CommandResponse<BrowserShellWriteRequest[]>>('list_traffic_browser_shell_write_requests')
  return expectCommandData('list_traffic_browser_shell_write_requests', response)
}

export async function queueBrowserShellWrite(
  sessionId: string,
  inputText: string,
  requiresApproval = true,
): Promise<BrowserShellWriteRequest> {
  const response = await invoke<CommandResponse<BrowserShellWriteRequest>>('queue_traffic_browser_shell_write', {
    payload: {
      sessionId,
      inputText,
      requiresApproval,
    },
  })
  return expectCommandData('queue_traffic_browser_shell_write', response)
}

export async function respondBrowserShellWrite(
  requestId: string,
  allowed: boolean,
): Promise<BrowserShellWriteRequest> {
  const response = await invoke<CommandResponse<BrowserShellWriteRequest>>('respond_traffic_browser_shell_write', {
    payload: {
      requestId,
      allowed,
    },
  })
  return expectCommandData('respond_traffic_browser_shell_write', response)
}
