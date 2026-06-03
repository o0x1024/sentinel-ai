export const DEFAULT_BOT_TRANSPORT = 'weixin'

export const SUPPORTED_BOT_TRANSPORTS = [
  'weixin',
  'feishu',
  'discord',
] as const

export type SupportedBotTransport = (typeof SUPPORTED_BOT_TRANSPORTS)[number]
