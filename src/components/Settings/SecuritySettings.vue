<template>
  <div class="security-settings">
    <!-- 安全状态概览 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-shield-alt" :class="getSecurityStatusColor()"></i>
        </div>
        <div class="stat-title">{{ t('settings.security.status') }}</div>
        <div class="stat-value text-sm" :class="getSecurityStatusColor()">
          {{ getSecurityStatusText() }}
        </div>
        <div class="stat-desc">{{ t('settings.security.score') }}: {{ getSecurityScore() }}/100</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-key"></i>
        </div>
        <div class="stat-title">{{ t('settings.security.encryption') }}</div>
        <div class="stat-value text-sm" :class="settings.security.encryption ? 'text-success' : 'text-warning'">
          {{ settings.security.encryption ? t('settings.enabled') : t('settings.disabled') }}
        </div>
        <div class="stat-desc">{{ t('settings.security.encryptionType') }}: {{ settings.security.encryptionType || 'AES-256' }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-user-shield"></i>
        </div>
        <div class="stat-title">{{ t('settings.security.authentication') }}</div>
        <div class="stat-value text-sm" :class="settings.security.requireAuth ? 'text-success' : 'text-warning'">
          {{ settings.security.requireAuth ? t('settings.enabled') : t('settings.disabled') }}
        </div>
        <div class="stat-desc">{{ t('settings.security.authMethod') }}: {{ getAuthMethodText() }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-history"></i>
        </div>
        <div class="stat-title">{{ t('settings.security.lastAudit') }}</div>
        <div class="stat-value text-sm">{{ formatDate(securityStatus.lastAudit) }}</div>
        <div class="stat-desc">{{ securityStatus.auditIssues || 0 }} {{ t('settings.security.issues') }}</div>
      </div>
    </div>

    <!-- 身份验证设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-user-lock"></i>
          {{ t('settings.security.authentication') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 基本认证设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.basicAuth') }}</h4>
            
            <!-- 启用身份验证 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.requireAuth') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.requireAuth">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.requireAuthHint') }}</span>
              </label>
            </div>
            
            <!-- 认证方式 -->
            <div v-if="settings.security.requireAuth" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.authMethod') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.authMethod">
                <option value="password">{{ t('settings.security.authMethods.password') }}</option>
                <option value="pin">{{ t('settings.security.authMethods.pin') }}</option>
                <option value="biometric">{{ t('settings.security.authMethods.biometric') }}</option>
                <option value="token">{{ t('settings.security.authMethods.token') }}</option>
              </select>
            </div>
            
            <!-- 密码设置 -->
            <div v-if="settings.security.requireAuth && settings.security.authMethod === 'password'" class="space-y-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.currentPassword') }}</span>
                </label>
                <input type="password" class="input input-bordered" 
                       v-model="passwordForm.current" 
                       :placeholder="t('settings.security.enterCurrentPassword')">
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.newPassword') }}</span>
                </label>
                <input type="password" class="input input-bordered" 
                       v-model="passwordForm.new" 
                       :placeholder="t('settings.security.enterNewPassword')">
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.confirmPassword') }}</span>
                </label>
                <input type="password" class="input input-bordered" 
                       v-model="passwordForm.confirm" 
                       :placeholder="t('settings.security.confirmNewPassword')">
              </div>
              
              <div class="form-control">
                <button class="btn btn-primary" @click="changePassword">
                  <i class="fas fa-key"></i>
                  {{ t('settings.security.changePassword') }}
                </button>
              </div>
            </div>
            
            <!-- PIN设置 -->
            <div v-if="settings.security.requireAuth && settings.security.authMethod === 'pin'" class="space-y-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.pin') }}</span>
                </label>
                <input type="password" class="input input-bordered" 
                       v-model="settings.security.pin" 
                       maxlength="6" 
                       :placeholder="t('settings.security.enterPin')">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.security.pinHint') }}</span>
                </label>
              </div>
            </div>
          </div>
          
          <!-- 高级认证设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.advancedAuth') }}</h4>
            
            <!-- 会话超时 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.sessionTimeout') }}</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1" 
                       v-model.number="settings.security.sessionTimeout" 
                       min="5" max="480" step="5">
                <span class="text-sm min-w-[80px]">{{ settings.security.sessionTimeout }}{{ t('settings.security.minutes') }}</span>
              </div>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.sessionTimeoutHint') }}</span>
              </label>
            </div>
            
            <!-- 最大登录尝试次数 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.maxLoginAttempts') }}</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="settings.security.maxLoginAttempts" 
                     min="3" max="10">
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.maxLoginAttemptsHint') }}</span>
              </label>
            </div>
            
            <!-- 锁定时间 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.lockoutDuration') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.lockoutDuration">
                <option value="5">5 {{ t('settings.security.minutes') }}</option>
                <option value="15">15 {{ t('settings.security.minutes') }}</option>
                <option value="30">30 {{ t('settings.security.minutes') }}</option>
                <option value="60">1 {{ t('settings.security.hour') }}</option>
                <option value="180">3 {{ t('settings.security.hours') }}</option>
                <option value="1440">24 {{ t('settings.security.hours') }}</option>
              </select>
            </div>
            
            <!-- 双因素认证 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.twoFactorAuth') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.twoFactorAuth">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.twoFactorAuthHint') }}</span>
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 数据加密设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-lock"></i>
          {{ t('settings.security.encryption') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 加密设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.encryptionSettings') }}</h4>
            
            <!-- 启用加密 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.enableEncryption') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.encryption">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.enableEncryptionHint') }}</span>
              </label>
            </div>
            
            <!-- 加密算法 -->
            <div v-if="settings.security.encryption" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.encryptionAlgorithm') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.encryptionType">
                <option value="AES-256">AES-256 ({{ t('settings.security.recommended') }})</option>
                <option value="AES-192">AES-192</option>
                <option value="AES-128">AES-128</option>
                <option value="ChaCha20">ChaCha20</option>
              </select>
            </div>
            
            <!-- 密钥管理 -->
            <div v-if="settings.security.encryption" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.keyManagement') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.keyManagement">
                <option value="auto">{{ t('settings.security.keyManagementAuto') }}</option>
                <option value="manual">{{ t('settings.security.keyManagementManual') }}</option>
                <option value="hardware">{{ t('settings.security.keyManagementHardware') }}</option>
              </select>
            </div>
            
            <!-- 密钥轮换 -->
            <div v-if="settings.security.encryption" class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.keyRotation') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.keyRotation">
              </label>
            </div>
            
            <!-- 轮换周期 -->
            <div v-if="settings.security.encryption && settings.security.keyRotation" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.rotationPeriod') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.rotationPeriod">
                <option value="7">7 {{ t('settings.security.days') }}</option>
                <option value="30">30 {{ t('settings.security.days') }}</option>
                <option value="90">90 {{ t('settings.security.days') }}</option>
                <option value="365">365 {{ t('settings.security.days') }}</option>
              </select>
            </div>
          </div>
          
          <!-- 加密范围 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.encryptionScope') }}</h4>
            
            <!-- 数据库加密 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.encryptDatabase') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.encryptDatabase">
              </label>
            </div>
            
            <!-- 配置文件加密 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.encryptConfig') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.encryptConfig">
              </label>
            </div>
            
            <!-- 日志加密 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.encryptLogs') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.encryptLogs">
              </label>
            </div>
            
            <!-- 缓存加密 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.encryptCache') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.encryptCache">
              </label>
            </div>
            
            <!-- 备份加密 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.encryptBackups') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.encryptBackups">
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 网络安全设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-network-wired"></i>
          {{ t('settings.security.networkSecurity') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 连接安全 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.connectionSecurity') }}</h4>
            
            <!-- 强制HTTPS -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.forceHTTPS') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.forceHTTPS">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.forceHTTPSHint') }}</span>
              </label>
            </div>
            
            <!-- 证书验证 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.verifyCertificates') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.verifyCertificates">
              </label>
            </div>
            
            <!-- 代理设置 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.useProxy') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.useProxy">
              </label>
            </div>
            
            <!-- 代理配置 -->
            <div v-if="settings.security.useProxy" class="space-y-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.proxyType') }}</span>
                </label>
                <select class="select select-bordered" v-model="settings.security.proxyType">
                  <option value="http">HTTP</option>
                  <option value="https">HTTPS</option>
                  <option value="socks5">SOCKS5</option>
                </select>
              </div>
              
              <div class="grid grid-cols-2 gap-3">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.security.proxyHost') }}</span>
                  </label>
                  <input type="text" class="input input-bordered" 
                         v-model="settings.security.proxyHost" 
                         placeholder="127.0.0.1">
                </div>
                
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.security.proxyPort') }}</span>
                  </label>
                  <input type="number" class="input input-bordered" 
                         v-model.number="settings.security.proxyPort" 
                         placeholder="8080">
                </div>
              </div>
            </div>
          </div>
          
          <!-- 访问控制 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.accessControl') }}</h4>
            
            <!-- IP白名单 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.enableIPWhitelist') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.enableIPWhitelist">
              </label>
            </div>
            
            <!-- 白名单配置 -->
            <div v-if="settings.security.enableIPWhitelist" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.allowedIPs') }}</span>
              </label>
              <textarea class="textarea textarea-bordered" 
                        v-model="settings.security.allowedIPs" 
                        :placeholder="t('settings.security.allowedIPsPlaceholder')"
                        rows="3"></textarea>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.security.allowedIPsHint') }}</span>
              </label>
            </div>
            
            <!-- 速率限制 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.enableRateLimit') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.enableRateLimit">
              </label>
            </div>
            
            <!-- 速率限制配置 -->
            <div v-if="settings.security.enableRateLimit" class="space-y-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.requestsPerMinute') }}</span>
                </label>
                <input type="number" class="input input-bordered" 
                       v-model.number="settings.security.requestsPerMinute" 
                       min="1" max="1000">
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.burstLimit') }}</span>
                </label>
                <input type="number" class="input input-bordered" 
                       v-model.number="settings.security.burstLimit" 
                       min="1" max="100">
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 审计和日志 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-clipboard-list"></i>
          {{ t('settings.security.auditLogging') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 审计设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.auditSettings') }}</h4>
            
            <!-- 启用审计 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.enableAudit') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.enableAudit">
              </label>
            </div>
            
            <!-- 审计级别 -->
            <div v-if="settings.security.enableAudit" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.auditLevel') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.auditLevel">
                <option value="basic">{{ t('settings.security.auditLevels.basic') }}</option>
                <option value="detailed">{{ t('settings.security.auditLevels.detailed') }}</option>
                <option value="comprehensive">{{ t('settings.security.auditLevels.comprehensive') }}</option>
              </select>
            </div>
            
            <!-- 审计事件 -->
            <div v-if="settings.security.enableAudit" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.auditEvents') }}</span>
              </label>
              <div class="space-y-2">
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.security.auditLogin') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.security.auditLogin">
                </label>
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.security.auditConfigChanges') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.security.auditConfigChanges">
                </label>
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.security.auditDataAccess') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.security.auditDataAccess">
                </label>
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.security.auditErrors') }}</span>
                  <input type="checkbox" class="checkbox checkbox-primary" 
                         v-model="settings.security.auditErrors">
                </label>
              </div>
            </div>
          </div>
          
          <!-- 日志设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.logSettings') }}</h4>
            
            <!-- 日志保留期 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.security.logRetention') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.security.logRetention">
                <option value="7">7 {{ t('settings.security.days') }}</option>
                <option value="30">30 {{ t('settings.security.days') }}</option>
                <option value="90">90 {{ t('settings.security.days') }}</option>
                <option value="365">365 {{ t('settings.security.days') }}</option>
                <option value="-1">{{ t('settings.security.forever') }}</option>
              </select>
            </div>
            
            <!-- 日志压缩 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.compressLogs') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.compressLogs">
              </label>
            </div>
            
            <!-- 远程日志 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.security.remoteLogging') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.security.remoteLogging">
              </label>
            </div>
            
            <!-- 远程日志配置 -->
            <div v-if="settings.security.remoteLogging" class="space-y-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.logServer') }}</span>
                </label>
                <input type="url" class="input input-bordered" 
                       v-model="settings.security.logServer" 
                       placeholder="https://logs.example.com">
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.security.logApiKey') }}</span>
                </label>
                <input type="password" class="input input-bordered" 
                       v-model="settings.security.logApiKey" 
                       :placeholder="t('settings.security.enterLogApiKey')">
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 安全操作 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-tools"></i>
          {{ t('settings.security.securityOperations') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 安全检查 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.securityCheck') }}</h4>
            
            <div class="form-control">
              <button class="btn btn-primary" @click="runSecurityAudit">
                <i class="fas fa-search"></i>
                {{ t('settings.security.runSecurityAudit') }}
              </button>
            </div>
            
            <div class="form-control">
              <button class="btn btn-info" @click="checkVulnerabilities">
                <i class="fas fa-bug"></i>
                {{ t('settings.security.checkVulnerabilities') }}
              </button>
            </div>
            
            <div class="form-control">
              <button class="btn btn-outline" @click="generateSecurityReport">
                <i class="fas fa-file-alt"></i>
                {{ t('settings.security.generateReport') }}
              </button>
            </div>
          </div>
          
          <!-- 紧急操作 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.security.emergencyOperations') }}</h4>
            
            <div class="form-control">
              <button class="btn btn-warning" @click="lockApplication">
                <i class="fas fa-lock"></i>
                {{ t('settings.security.lockApplication') }}
              </button>
            </div>
            
            <div class="form-control">
              <button class="btn btn-error" @click="emergencyShutdown">
                <i class="fas fa-power-off"></i>
                {{ t('settings.security.emergencyShutdown') }}
              </button>
            </div>
            
            <div class="form-control">
              <button class="btn btn-error" @click="wipeSecurityData">
                <i class="fas fa-eraser"></i>
                {{ t('settings.security.wipeSecurityData') }}
              </button>
              <label class="label">
                <span class="label-text-alt text-error">{{ t('settings.security.wipeSecurityDataWarning') }}</span>
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 保存按钮 -->
    <div class="flex justify-end">
      <button class="btn btn-primary" @click="saveSecurityConfig" :disabled="saving">
        <i class="fas fa-save"></i>
        {{ saving ? t('settings.saving') : t('settings.security.saveConfig') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// Props
interface Props {
  securityStatus: any
  settings: any
  saving: boolean
}

const props = defineProps<Props>()

// Emits
interface Emits {
  'update:settings': [value: any]
  'changePassword': [passwordForm: any]
  'runSecurityAudit': []
  'checkVulnerabilities': []
  'generateSecurityReport': []
  'lockApplication': []
  'emergencyShutdown': []
  'wipeSecurityData': []
  'saveSecurityConfig': []
}

const emit = defineEmits<Emits>()

// Reactive data
const passwordForm = reactive({
  current: '',
  new: '',
  confirm: ''
})

// Computed
const settings = computed({
  get: () => props.settings,
  set: (value: any) => emit('update:settings', value)
})

// Methods
const formatDate = (date: string | null) => {
  if (!date) return t('settings.security.never')
  return new Date(date).toLocaleDateString()
}

const getSecurityStatusColor = () => {
  const score = getSecurityScore()
  if (score >= 80) return 'text-success'
  if (score >= 60) return 'text-warning'
  return 'text-error'
}

const getSecurityStatusText = () => {
  const score = getSecurityScore()
  if (score >= 80) return t('settings.security.statusGood')
  if (score >= 60) return t('settings.security.statusWarning')
  return t('settings.security.statusCritical')
}

const getSecurityScore = () => {
  let score = 0
  
  // 基础分数
  if (props.settings.security.requireAuth) score += 20
  if (props.settings.security.encryption) score += 25
  if (props.settings.security.twoFactorAuth) score += 15
  if (props.settings.security.enableAudit) score += 10
  if (props.settings.security.forceHTTPS) score += 10
  if (props.settings.security.verifyCertificates) score += 5
  if (props.settings.security.enableIPWhitelist) score += 5
  if (props.settings.security.enableRateLimit) score += 5
  if (props.settings.security.keyRotation) score += 5
  
  return Math.min(score, 100)
}

const getAuthMethodText = () => {
  const methods: Record<string, string> = {
    'password': t('settings.security.authMethods.password'),
    'pin': t('settings.security.authMethods.pin'),
    'biometric': t('settings.security.authMethods.biometric'),
    'token': t('settings.security.authMethods.token')
  }
  return methods[props.settings.security.authMethod] || t('settings.security.authMethods.password')
}

const changePassword = () => {
  emit('changePassword', { ...passwordForm })
  // 清空表单
  passwordForm.current = ''
  passwordForm.new = ''
  passwordForm.confirm = ''
}

const runSecurityAudit = () => {
  emit('runSecurityAudit')
}

const checkVulnerabilities = () => {
  emit('checkVulnerabilities')
}

const generateSecurityReport = () => {
  emit('generateSecurityReport')
}

const lockApplication = () => {
  emit('lockApplication')
}

const emergencyShutdown = () => {
  emit('emergencyShutdown')
}

const wipeSecurityData = () => {
  emit('wipeSecurityData')
}

const saveSecurityConfig = () => {
  emit('saveSecurityConfig')
}
</script>

<style scoped>
.security-settings {
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

.btn-warning {
  @apply transition-all duration-200;
}

.btn-warning:hover {
  @apply scale-105 shadow-lg;
}
</style>