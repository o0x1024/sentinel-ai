import { invoke } from '@tauri-apps/api/core'

export interface WeixinGatewayConfig {
  enabled: boolean
  account_id: string
  token: string
  base_url: string
  assistant_profile_id: string | null
  dm_policy: string
  allowed_users: string[]
  group_policy: string
  group_allowed_users: string[]
  default_service_name: string
  max_iterations: number
  timeout_secs: number
}

export interface WeixinGatewayStatus {
  running: boolean
  account_id: string | null
  started_at: string | null
  last_error: string | null
  last_message_at: string | null
}

export interface WeixinQrLoginResponse {
  qrcode: string
  qrcode_img_content: string
  scan_data: string
}

export interface WeixinQrLoginStatus {
  status: string
  account_id: string | null
  token_configured: boolean
  base_url: string | null
  user_id: string | null
  message: string | null
}

export async function getWeixinGatewayConfig(): Promise<WeixinGatewayConfig> {
  return await invoke<WeixinGatewayConfig>('get_weixin_gateway_config')
}

export async function saveWeixinGatewayConfig(config: WeixinGatewayConfig): Promise<void> {
  await invoke('save_weixin_gateway_config', { config })
}

export async function startWeixinGateway(config?: WeixinGatewayConfig): Promise<string> {
  return await invoke<string>('start_weixin_gateway', { config })
}

export async function stopWeixinGateway(): Promise<string> {
  return await invoke<string>('stop_weixin_gateway')
}

export async function getWeixinGatewayStatus(): Promise<WeixinGatewayStatus> {
  return await invoke<WeixinGatewayStatus>('get_weixin_gateway_status')
}

export async function createWeixinQrLogin(): Promise<WeixinQrLoginResponse> {
  return await invoke<WeixinQrLoginResponse>('create_weixin_qr_login', { botType: '3', bot_type: '3' })
}

export async function pollWeixinQrLogin(
  qrcode: string,
  baseUrl?: string,
): Promise<WeixinQrLoginStatus> {
  return await invoke<WeixinQrLoginStatus>('poll_weixin_qr_login', { qrcode, baseUrl, base_url: baseUrl })
}
