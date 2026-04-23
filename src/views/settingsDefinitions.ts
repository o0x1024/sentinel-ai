export interface DatabaseConfig {
  db_type: string
  path?: string
  host?: string
  port?: number
  database?: string
  username?: string
  password?: string
  enable_wal?: boolean
  enable_ssl?: boolean
  max_connections?: number
  query_timeout?: number
}

export const OUTPUT_STORAGE_THRESHOLD_MIN = 8000
export const OUTPUT_STORAGE_THRESHOLD_RECOMMENDED_MAX = 32000
export const OUTPUT_STORAGE_THRESHOLD_DEFAULT = 16000

export const settingsCategories = [
  { id: 'ai', icon: 'fas fa-robot' },
  { id: 'rag', icon: 'fas fa-database' },
  { id: 'database', icon: 'fas fa-server' },
  { id: 'system', icon: 'fas fa-cog' },
  { id: 'security', icon: 'fas fa-shield-alt' },
  { id: 'network', icon: 'fas fa-network-wired' },
]

export const createDefaultSettings = () => ({
  ai: {
    temperature: 0.7,
    maxTokens: 2000,
    toolOutputLimit: 50000,
    outputStorageThreshold: OUTPUT_STORAGE_THRESHOLD_DEFAULT,
    maxTurns: 100,
  },
  database: {
    type: 'sqlite',
    path: '',
    host: 'localhost',
    port: 5432,
    name: 'sentinel_ai',
    username: '',
    password: '',
    maxConnections: 10,
    queryTimeout: 30,
    enableWAL: true,
    enableSSL: false,
    autoBackup: true,
    backupFrequency: 'daily',
    backupRetention: 7,
    backupPath: '',
    autoCleanup: false,
    retentionDays: 30,
    cleanupLogs: true,
    cleanupTempFiles: true,
    cleanupOldSessions: true,
  },
  general: {
    theme: 'auto',
    darkMode: false,
    fontSize: 16,
    compactMode: false,
    language: 'auto',
    region: 'auto',
    timezone: 'auto',
    dateFormat: 'YYYY-MM-DD',
    autoStart: false,
    startMinimized: false,
    restoreSession: true,
    checkUpdates: true,
    closeAction: 'minimize',
    closeToTray: false,
    minimizeToTray: false,
    alwaysOnTop: false,
    windowOpacity: 1,
    memoryLimit: 2048,
    autoGC: true,
    preload: false,
    maxConnections: 5,
    requestTimeout: 30,
    retryCount: 3,
    analytics: false,
    errorReporting: true,
    usageStats: false,
    encryptLocalData: true,
    uiScale: 100,
  },
  system: {
    autoStart: false,
    minimizeToTray: true,
  },
  security: {
    requireAuth: false,
    authMethod: 'password',
    sessionTimeout: 30,
    maxLoginAttempts: 5,
    lockoutDuration: 15,
    twoFactorAuth: false,
    encryption: true,
    encryptionType: 'AES-256',
    keyManagement: 'auto',
    keyRotation: true,
    rotationPeriod: 90,
    encryptDatabase: true,
    encryptConfig: true,
    encryptLogs: false,
    encryptCache: false,
    encryptBackups: true,
    forceHTTPS: true,
    verifyCertificates: true,
    useProxy: false,
    proxyType: 'http',
    proxyHost: '',
    proxyPort: 8080,
    enableIPWhitelist: false,
    allowedIPs: '',
    enableRateLimit: false,
    requestsPerMinute: 60,
    burstLimit: 10,
    logRetention: 90,
    compressLogs: true,
    remoteLogging: false,
    logServer: '',
    logApiKey: '',
    pin: '',
  },
})

export const createDefaultRagConfig = () => ({
  embedding_provider: 'ollama',
  embedding_model: 'nomic-embed-text',
  embedding_dimensions: null,
  embedding_api_key: '',
  embedding_base_url: 'http://localhost:11434',
  chunk_size_chars: 1000,
  chunk_overlap_chars: 200,
  chunking_strategy: 'RecursiveCharacter',
  min_chunk_size_chars: 100,
  max_chunk_size_chars: 3000,
  top_k: 5,
  mmr_lambda: 0.7,
  similarity_threshold: 0.7,
  batch_size: 10,
  max_concurrent: 4,
  reranking_enabled: false,
  reranking_provider: '',
  reranking_model: '',
  augmentation_enabled: false,
})

export const createDefaultCustomProvider = () => ({
  name: '',
  api_key: '',
  api_base: '',
  model_id: '',
  display_name: '',
  rig_provider: '',
  compat_mode: 'openai',
  extra_headers_json: '',
  timeout: 120,
  max_retries: 3,
})
