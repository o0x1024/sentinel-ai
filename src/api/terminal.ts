/**
 * Terminal API - Frontend interface for interactive terminal commands
 */

import { invoke } from '@tauri-apps/api/core'

/**
 * Terminal server configuration
 */
export interface TerminalServerConfig {
  host: string
  port: number
}

/**
 * Terminal server status
 */
export interface TerminalServerStatus {
  running: boolean
  session_count: number
}

/**
 * Terminal session state
 */
export type SessionState = 'Starting' | 'Running' | 'Stopped' | 'Error'

/**
 * Execution mode for terminal session
 */
export type ExecutionMode = 'docker' | 'host'

/**
 * Terminal session information
 */
export interface SessionInfo {
  id: string
  state: SessionState
  last_activity: number
  execution_mode: ExecutionMode
}

/**
 * Start the terminal WebSocket server
 * @param config Optional server configuration (default: 127.0.0.1:8765)
 * @returns Success message
 */
export async function startTerminalServer(
  config?: TerminalServerConfig
): Promise<string> {
  return await invoke<string>('start_terminal_server', { config })
}

/**
 * Stop the terminal WebSocket server
 * @returns Success message
 */
export async function stopTerminalServer(): Promise<string> {
  return await invoke<string>('stop_terminal_server')
}

/**
 * Get terminal server status
 * @returns Server status with session count
 */
export async function getTerminalServerStatus(): Promise<TerminalServerStatus> {
  return await invoke<TerminalServerStatus>('get_terminal_server_status')
}

/**
 * List all active terminal sessions
 * @returns Array of session information
 */
export async function listTerminalSessions(): Promise<SessionInfo[]> {
  return await invoke<SessionInfo[]>('list_terminal_sessions')
}

/**
 * Stop a specific terminal session
 * @param sessionId Session ID to stop
 * @returns Success message
 */
export async function stopTerminalSession(sessionId: string): Promise<string> {
  return await invoke<string>('stop_terminal_session', { sessionId })
}

/**
 * Get the WebSocket URL for connecting to the terminal server
 * @returns WebSocket URL (e.g., ws://127.0.0.1:8765)
 */
export async function getTerminalWebSocketUrl(): Promise<string> {
  return await invoke<string>('get_terminal_websocket_url')
}

/**
 * Terminal API namespace
 */
export const TerminalAPI = {
  startServer: startTerminalServer,
  stopServer: stopTerminalServer,
  getStatus: getTerminalServerStatus,
  listSessions: listTerminalSessions,
  stopSession: stopTerminalSession,
  getWebSocketUrl: getTerminalWebSocketUrl,
}

export default TerminalAPI
