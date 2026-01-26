<template>
  <div class="database-settings">
    <!-- 数据库状态概览 -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-database"></i>
        </div>
        <div class="stat-title">{{ t('settings.database.status') }}</div>
        <div class="stat-value text-sm" :class="databaseStatus.connected ? 'text-success' : 'text-error'">
          {{ databaseStatus.connected ? t('settings.database.connected') : t('settings.database.disconnected') }}
        </div>
        <div class="stat-desc">{{ t('settings.database.type') }}: {{ databaseStatus.type || 'SQLite' }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-hdd"></i>
        </div>
        <div class="stat-title">{{ t('settings.database.size') }}</div>
        <div class="stat-value text-sm">{{ formatFileSize(databaseStatus.size || 0) }}</div>
        <div class="stat-desc">{{ databaseStatus.tables || 0 }} {{ t('settings.database.tables') }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-clock"></i>
        </div>
        <div class="stat-title">{{ t('settings.database.lastBackup') }}</div>
        <div class="stat-value text-sm">{{ formatDate(databaseStatus.lastBackup) }}</div>
        <div class="stat-desc">{{ t('settings.database.autoBackup') }}: {{ settings.database.autoBackup ? t('settings.enabled') : t('settings.disabled') }}</div>
      </div>
    </div>

    <!-- 数据库配置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-cog"></i>
          {{ t('settings.database.configuration') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 基本设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.database.basicSettings') }}</h4>
            
            <!-- 数据库类型 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.type') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.database.type">
                <option value="sqlite">SQLite ({{ t('settings.database.recommended') }})</option>
                <option value="postgresql">PostgreSQL</option>
                <option value="mysql">MySQL</option>
              </select>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.database.typeHint') }}</span>
              </label>
            </div>
            
            <!-- 数据库路径 (SQLite) -->
            <div v-if="settings.database.type === 'sqlite'" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.path') }}</span>
              </label>
              <div class="input-group">
                <input type="text" class="input input-bordered flex-1" 
                       v-model="settings.database.path" readonly>
                <button class="btn btn-outline" @click="selectDatabasePath">
                  <i class="fas fa-folder-open"></i>
                  {{ t('settings.database.browse') }}
                </button>
              </div>
            </div>
            
            <!-- 连接设置 (PostgreSQL/MySQL) -->
            <div v-else class="space-y-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.database.host') }}</span>
                </label>
                <input type="text" class="input input-bordered" 
                       v-model="settings.database.host" placeholder="localhost">
              </div>
              
              <div class="grid grid-cols-2 gap-3">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.database.port') }}</span>
                  </label>
                  <input type="number" class="input input-bordered" 
                         v-model.number="settings.database.port" 
                         :placeholder="settings.database.type === 'postgresql' ? '5432' : '3306'">
                </div>
                
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.database.name') }}</span>
                  </label>
                  <input type="text" class="input input-bordered" 
                         v-model="settings.database.name" placeholder="sentinel_ai">
                </div>
              </div>
              
              <div class="grid grid-cols-2 gap-3">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.database.username') }}</span>
                  </label>
                  <input type="text" class="input input-bordered" 
                         v-model="settings.database.username">
                </div>
                
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.database.password') }}</span>
                  </label>
                  <input type="password" class="input input-bordered" 
                         v-model="settings.database.password">
                </div>
              </div>
            </div>
            
            <!-- 测试连接 -->
            <div class="form-control">
              <button class="btn btn-outline" @click="testDatabaseConnection">
                <i class="fas fa-plug"></i>
                {{ t('settings.database.testConnection') }}
              </button>
            </div>
          </div>
          
          <!-- 高级设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.database.advancedSettings') }}</h4>
            
            <!-- 连接池设置 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.maxConnections') }}</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="settings.database.maxConnections" 
                     min="1" max="100">
              <label class="label">
                <span class="label-text-alt">{{ t('settings.database.maxConnectionsHint') }}</span>
              </label>
            </div>
            
            <!-- 查询超时 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.queryTimeout') }}</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1" 
                       v-model.number="settings.database.queryTimeout" 
                       min="5" max="60" step="5">
                <span class="text-sm min-w-[60px]">{{ settings.database.queryTimeout }}s</span>
              </div>
            </div>
            
            <!-- 启用WAL模式 (SQLite) -->
            <div v-if="settings.database.type === 'sqlite'" class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.database.enableWAL') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.database.enableWAL">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.database.enableWALHint') }}</span>
              </label>
            </div>
            
            <!-- 启用SSL (PostgreSQL/MySQL) -->
            <div v-else class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.database.enableSSL') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.database.enableSSL">
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 备份与恢复 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-shield-alt"></i>
          {{ t('settings.database.backupRestore') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 备份设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.database.backupSettings') }}</h4>
            
            <!-- 自动备份 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.database.autoBackup') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.database.autoBackup">
              </label>
            </div>
            
            <!-- 备份频率 -->
            <div v-if="settings.database.autoBackup" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.backupFrequency') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.database.backupFrequency">
                <option value="daily">{{ t('settings.database.daily') }}</option>
                <option value="weekly">{{ t('settings.database.weekly') }}</option>
                <option value="monthly">{{ t('settings.database.monthly') }}</option>
              </select>
            </div>
            
            <!-- 备份保留数量 -->
            <div v-if="settings.database.autoBackup" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.backupRetention') }}</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="settings.database.backupRetention" 
                     min="1" max="30">
              <label class="label">
                <span class="label-text-alt">{{ t('settings.database.backupRetentionHint') }}</span>
              </label>
            </div>
            
            <!-- 备份路径 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.backupPath') }}</span>
              </label>
              <div class="input-group">
                <input type="text" class="input input-bordered flex-1" 
                       v-model="settings.database.backupPath" readonly>
                <button class="btn btn-outline" @click="selectBackupPath">
                  <i class="fas fa-folder-open"></i>
                  {{ t('settings.database.browse') }}
                </button>
              </div>
            </div>
          </div>
          
          <!-- 备份操作 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.database.backupOperations') }}</h4>
            
            <!-- 立即备份 -->
            <div class="form-control">
              <button class="btn btn-primary" @click="createBackup">
                <i class="fas fa-download"></i>
                {{ t('settings.database.createBackup') }}
              </button>
            </div>
            
            <!-- 恢复备份 -->
            <div class="form-control">
              <button class="btn btn-warning" @click="selectBackupFile">
                <i class="fas fa-upload"></i>
                {{ t('settings.database.restoreBackup') }}
              </button>
            </div>
            
            <!-- 导出数据 -->
            <div class="form-control">
              <button class="btn btn-outline" @click="exportData">
                <i class="fas fa-file-export"></i>
                {{ t('settings.database.exportData') }}
              </button>
            </div>
            
            <!-- 导入数据 -->
            <div class="form-control">
              <button class="btn btn-outline" @click="importData">
                <i class="fas fa-file-import"></i>
                {{ t('settings.database.importData') }}
              </button>
            </div>
            
            <!-- 数据库迁移 -->
            <div class="form-control">
              <button class="btn btn-info" @click="migrateDatabase">
                <i class="fas fa-exchange-alt"></i>
                {{ t('settings.database.migrateDatabase') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 数据管理 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-broom"></i>
          {{ t('settings.database.dataManagement') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 清理设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.database.cleanupSettings') }}</h4>
            
            <!-- 自动清理 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.database.autoCleanup') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.database.autoCleanup">
              </label>
            </div>
            
            <!-- 保留天数 -->
            <div v-if="settings.database.autoCleanup" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.retentionDays') }}</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="settings.database.retentionDays" 
                     min="1" max="365">
              <label class="label">
                <span class="label-text-alt">{{ t('settings.database.retentionDaysHint') }}</span>
              </label>
            </div>
            
            <!-- 清理类型 -->
            <div v-if="settings.database.autoCleanup" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.database.cleanupTypes') }}</span>
              </label>
              <div class="space-y-2">
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.database.cleanupLogs') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.database.cleanupLogs">
                </label>
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.database.cleanupTempFiles') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.database.cleanupTempFiles">
                </label>
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.database.cleanupOldSessions') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.database.cleanupOldSessions">
                </label>
              </div>
            </div>
          </div>
          
          <!-- 清理操作 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.database.cleanupOperations') }}</h4>
            
            <!-- 立即清理 -->
            <div class="form-control">
              <button class="btn btn-warning" @click="cleanupNow">
                <i class="fas fa-broom"></i>
                {{ t('settings.database.cleanupNow') }}
              </button>
            </div>
            
            <!-- 优化数据库 -->
            <div class="form-control">
              <button class="btn btn-info" @click="optimizeDatabase">
                <i class="fas fa-tachometer-alt"></i>
                {{ t('settings.database.optimize') }}
              </button>
            </div>
            
            <!-- 重建索引 -->
            <div class="form-control">
              <button class="btn btn-outline" @click="rebuildIndexes">
                <i class="fas fa-hammer"></i>
                {{ t('settings.database.rebuildIndexes') }}
              </button>
            </div>
            
            <!-- 危险操作：重置数据库 -->
            <div class="divider">{{ t('settings.database.dangerZone') }}</div>
            
            <div class="form-control">
              <button class="btn btn-error" @click="resetDatabase">
                <i class="fas fa-exclamation-triangle"></i>
                {{ t('settings.database.resetDatabase') }}
              </button>
              <label class="label">
                <span class="label-text-alt text-error">{{ t('settings.database.resetDatabaseWarning') }}</span>
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>


  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// Props
interface Props {
  databaseStatus: any
  settings: any
  saving: boolean
}

const props = defineProps<Props>()

// Emits
interface Emits {
  'update:settings': [value: any]
  'selectDatabasePath': []
  'selectBackupPath': []
  'testDatabaseConnection': []
  'createBackup': []
  'selectBackupFile': []
  'exportData': []
  'importData': []
  'migrateDatabase': []
  'cleanupNow': []
  'optimizeDatabase': []
  'rebuildIndexes': []
  'resetDatabase': []
  'saveDatabaseConfig': []
}

const emit = defineEmits<Emits>()

// Computed
const settings = computed({
  get: () => props.settings,
  set: (value: any) => emit('update:settings', value)
})

// Methods
const formatFileSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const formatDate = (date: string | null) => {
  if (!date) return t('settings.database.never')
  return new Date(date).toLocaleDateString()
}

const selectDatabasePath = () => {
  emit('selectDatabasePath')
}

const selectBackupPath = () => {
  emit('selectBackupPath')
}

const testDatabaseConnection = () => {
  emit('testDatabaseConnection')
}

const createBackup = () => {
  emit('createBackup')
}

const selectBackupFile = () => {
  emit('selectBackupFile')
}

const exportData = () => {
  emit('exportData')
}

const importData = () => {
  emit('importData')
}

const migrateDatabase = () => {
  emit('migrateDatabase')
}

const cleanupNow = () => {
  emit('cleanupNow')
}

const optimizeDatabase = () => {
  emit('optimizeDatabase')
}

const rebuildIndexes = () => {
  emit('rebuildIndexes')
}

// 自动保存配置
import { watch } from 'vue'
watch(() => props.settings.database, () => {
  emit('saveDatabaseConfig')
}, { deep: true })

const resetDatabase = () => {
  emit('resetDatabase')
}

const saveDatabaseConfig = () => {
  emit('saveDatabaseConfig')
}
</script>

<style scoped>
.database-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.stat {
  @apply transition-all duration-200 hover:scale-105;
}

.btn-error {
  @apply transition-all duration-200;
}

.btn-error:hover {
  @apply scale-105 shadow-lg;
}
</style>