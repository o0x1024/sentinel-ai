import { invoke } from '@tauri-apps/api/core'

export interface HttpGatewayConfig {
  enabled: boolean
  host: string
  port: number
  allow_lan: boolean
  cors: {
    enabled: boolean
    origins: string[]
  }
  auth: {
    required: boolean
    api_keys: string[]
    header_name: string
  }
  remote: {
    enabled: boolean
    mode: string
    public_base_url: string
  }
  limits: {
    max_body_bytes: number
    requests_per_minute: number
    max_concurrent_requests: number
  }
  audit: {
    enabled: boolean
    log_auth_failures: boolean
  }
}

export interface HttpGatewayStatus {
  running: boolean
  bind_addr: string | null
  started_at: string | null
  last_error: string | null
}

export async function getHttpGatewayConfig(): Promise<HttpGatewayConfig> {
  return await invoke<HttpGatewayConfig>('get_http_gateway_config')
}

export async function saveHttpGatewayConfig(config: HttpGatewayConfig): Promise<void> {
  await invoke('save_http_gateway_config', { config })
}

export async function startHttpGateway(config?: HttpGatewayConfig): Promise<string> {
  return await invoke<string>('start_http_gateway', { config })
}

export async function stopHttpGateway(): Promise<string> {
  return await invoke<string>('stop_http_gateway')
}

export async function getHttpGatewayStatus(): Promise<HttpGatewayStatus> {
  return await invoke<HttpGatewayStatus>('get_http_gateway_status')
}

export async function rotateHttpGatewayApiKey(): Promise<string> {
  return await invoke<string>('rotate_http_gateway_api_key')
}
