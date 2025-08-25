<template>
  <div class="agent-flow-test-panel">
    <div class="header">
      <h2>🔬 Agent执行流程测试</h2>
      <p class="description">
        端到端测试Agent系统的完整执行流程，验证从初始化到工具执行的整个链路
      </p>
    </div>

    <div class="test-sections">
      <!-- 完整流程测试 -->
      <div class="test-section">
        <h3>📋 完整流程测试</h3>
        <p>测试Agent管理器、引擎注册、工具系统集成和任务执行</p>
        
        <button 
          @click="runCompleteFlowTest" 
          :disabled="loading.completeFlow"
          class="test-button primary"
        >
          <span v-if="loading.completeFlow">🔄 测试中...</span>
          <span v-else>🚀 运行完整流程测试</span>
        </button>

        <div v-if="results.completeFlow" class="test-result" :class="results.completeFlow.success ? 'success' : 'error'">
          <div class="result-header">
            <span class="status-icon">{{ results.completeFlow.success ? '✅' : '❌' }}</span>
            <span class="stage">{{ results.completeFlow.stage }}</span>
            <span class="time">{{ results.completeFlow.execution_time_ms }}ms</span>
          </div>
          <div class="result-message">{{ results.completeFlow.message }}</div>
          <div v-if="results.completeFlow.details" class="result-details">
            <pre>{{ JSON.stringify(results.completeFlow.details, null, 2) }}</pre>
          </div>
        </div>
      </div>

      <!-- 工具系统测试 -->
      <div class="test-section">
        <h3>🛠️ 工具系统测试</h3>
        <p>检查工具系统的可用性和工具注册情况</p>
        
        <button 
          @click="runToolSystemTest" 
          :disabled="loading.toolSystem"
          class="test-button secondary"
        >
          <span v-if="loading.toolSystem">🔄 检查中...</span>
          <span v-else>🔍 检查工具系统</span>
        </button>

        <div v-if="results.toolSystem" class="test-result success">
          <div class="result-header">
            <span class="status-icon">📊</span>
            <span>工具系统状态</span>
            <span class="time">{{ results.toolSystem.execution_time_ms }}ms</span>
          </div>
          <div class="tools-summary">
            <div class="metric">
              <span class="label">总工具数:</span>
              <span class="value">{{ results.toolSystem.total_tools }}</span>
            </div>
            <div class="metric">
              <span class="label">可用工具:</span>
              <span class="value success">{{ results.toolSystem.available_tools }}</span>
            </div>
          </div>
          <div class="tools-list">
            <div v-for="tool in results.toolSystem.tools" :key="tool.name" class="tool-item">
              <span class="tool-name">{{ tool.name }}</span>
              <span class="tool-category">{{ tool.category }}</span>
              <span class="tool-status" :class="tool.available ? 'available' : 'unavailable'">
                {{ tool.available ? '✅' : '❌' }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- 工具执行测试 -->
      <div class="test-section">
        <h3>⚡ 工具执行测试</h3>
        <p>测试实际工具的执行能力</p>
        
        <div class="test-controls">
          <div class="input-group">
            <label>工具名称:</label>
            <select v-model="toolTest.toolName" class="form-control">
              <option value="port_scan">端口扫描 (port_scan)</option>
              <option value="r_subdomain">子域名扫描 (r_subdomain)</option>
            </select>
          </div>
          <div class="input-group">
            <label>目标:</label>
            <input 
              v-model="toolTest.target" 
              type="text" 
              placeholder="127.0.0.1" 
              class="form-control"
            >
          </div>
          <button 
            @click="runToolExecutionTest" 
            :disabled="loading.toolExecution"
            class="test-button accent"
          >
            <span v-if="loading.toolExecution">⚡ 执行中...</span>
            <span v-else">🔧 执行工具测试</span>
          </button>
        </div>

        <div v-if="results.toolExecution" class="test-result" :class="results.toolExecution.success ? 'success' : 'error'">
          <div class="result-header">
            <span class="status-icon">{{ results.toolExecution.success ? '✅' : '❌' }}</span>
            <span>{{ results.toolExecution.tool_name }} → {{ results.toolExecution.target }}</span>
            <span class="time">{{ results.toolExecution.execution_time_ms }}ms</span>
          </div>
          <div v-if="results.toolExecution.success && results.toolExecution.result" class="tool-result">
            <div class="metric">
              <span class="label">执行ID:</span>
              <span class="value">{{ results.toolExecution.result.execution_id }}</span>
            </div>
            <div class="metric">
              <span class="label">工具执行时间:</span>
              <span class="value">{{ results.toolExecution.result.duration_ms }}ms</span>
            </div>
            <div class="metric">
              <span class="label">成功状态:</span>
              <span class="value" :class="results.toolExecution.result.success ? 'success' : 'error'">
                {{ results.toolExecution.result.success ? '成功' : '失败' }}
              </span>
            </div>
            <div v-if="results.toolExecution.result.output" class="tool-output">
              <h4>工具输出:</h4>
              <pre>{{ JSON.stringify(results.toolExecution.result.output, null, 2) }}</pre>
            </div>
            <div v-if="results.toolExecution.result.error" class="tool-error">
              <h4>错误信息:</h4>
              <pre>{{ results.toolExecution.result.error }}</pre>
            </div>
          </div>
          <div v-if="!results.toolExecution.success" class="error-message">
            {{ results.toolExecution.error }}
          </div>
        </div>
      </div>
    </div>

    <!-- 总结面板 -->
    <div v-if="hasAnyResults" class="summary-panel">
      <h3>📈 测试总结</h3>
      <div class="summary-metrics">
        <div class="metric-card" :class="getOverallStatus()">
          <div class="metric-title">总体状态</div>
          <div class="metric-value">{{ getOverallStatusText() }}</div>
        </div>
        <div class="metric-card">
          <div class="metric-title">已执行测试</div>
          <div class="metric-value">{{ getCompletedTestCount() }}/3</div>
        </div>
        <div class="metric-card">
          <div class="metric-title">成功率</div>
          <div class="metric-value">{{ getSuccessRate() }}%</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// 响应式数据
const loading = ref({
  completeFlow: false,
  toolSystem: false,
  toolExecution: false
})

const results = ref({
  completeFlow: null as any,
  toolSystem: null as any,
  toolExecution: null as any
})

const toolTest = ref({
  toolName: 'port_scan',
  target: '127.0.0.1'
})

// 计算属性
const hasAnyResults = computed(() => 
  results.value.completeFlow || results.value.toolSystem || results.value.toolExecution
)

// 方法
async function runCompleteFlowTest() {
  loading.value.completeFlow = true
  try {
    const result = await invoke('test_complete_agent_flow')
    results.value.completeFlow = result
  } catch (error) {
    results.value.completeFlow = {
      success: false,
      stage: 'invoke_error',
      message: `调用失败: ${error}`,
      details: null,
      execution_time_ms: 0
    }
  } finally {
    loading.value.completeFlow = false
  }
}

async function runToolSystemTest() {
  loading.value.toolSystem = true
  try {
    const result = await invoke('test_tool_system_availability')
    results.value.toolSystem = result
  } catch (error) {
    console.error('Tool system test failed:', error)
  } finally {
    loading.value.toolSystem = false
  }
}

async function runToolExecutionTest() {
  loading.value.toolExecution = true
  try {
    const result = await invoke('test_tool_execution', {
      toolName: toolTest.value.toolName,
      target: toolTest.value.target || undefined
    })
    results.value.toolExecution = result
  } catch (error) {
    results.value.toolExecution = {
      success: false,
      tool_name: toolTest.value.toolName,
      target: toolTest.value.target,
      execution_time_ms: 0,
      error: `调用失败: ${error}`
    }
  } finally {
    loading.value.toolExecution = false
  }
}

function getCompletedTestCount(): number {
  let count = 0
  if (results.value.completeFlow) count++
  if (results.value.toolSystem) count++
  if (results.value.toolExecution) count++
  return count
}

function getSuccessRate(): number {
  const completed = getCompletedTestCount()
  if (completed === 0) return 0
  
  let successful = 0
  if (results.value.completeFlow?.success) successful++
  if (results.value.toolSystem) successful++ // 工具系统测试总是成功的
  if (results.value.toolExecution?.success) successful++
  
  return Math.round((successful / completed) * 100)
}

function getOverallStatus(): string {
  const rate = getSuccessRate()
  if (rate === 100) return 'success'
  if (rate >= 50) return 'warning'
  return 'error'
}

function getOverallStatusText(): string {
  const rate = getSuccessRate()
  if (rate === 100) return '全部通过'
  if (rate >= 50) return '部分通过'
  return '存在问题'
}
</script>

<style scoped>
.agent-flow-test-panel {
  padding: 20px;
  max-width: 1200px;
  margin: 0 auto;
}

.header {
  text-align: center;
  margin-bottom: 30px;
}

.header h2 {
  color: #2c3e50;
  margin-bottom: 10px;
}

.description {
  color: #666;
  font-size: 14px;
}

.test-sections {
  display: grid;
  gap: 20px;
  margin-bottom: 30px;
}

.test-section {
  border: 1px solid #e1e8ed;
  border-radius: 12px;
  padding: 20px;
  background: white;
}

.test-section h3 {
  color: #2c3e50;
  margin-bottom: 8px;
}

.test-section p {
  color: #666;
  margin-bottom: 15px;
  font-size: 14px;
}

.test-button {
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.test-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.test-button.primary {
  background: #3b82f6;
  color: white;
}

.test-button.primary:hover:not(:disabled) {
  background: #2563eb;
}

.test-button.secondary {
  background: #10b981;
  color: white;
}

.test-button.secondary:hover:not(:disabled) {
  background: #059669;
}

.test-button.accent {
  background: #f59e0b;
  color: white;
}

.test-button.accent:hover:not(:disabled) {
  background: #d97706;
}

.test-result {
  margin-top: 15px;
  padding: 15px;
  border-radius: 8px;
  border: 1px solid;
}

.test-result.success {
  background: #f0fdf4;
  border-color: #22c55e;
}

.test-result.error {
  background: #fef2f2;
  border-color: #ef4444;
}

.result-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
  font-weight: 500;
}

.status-icon {
  font-size: 16px;
}

.time {
  margin-left: auto;
  font-size: 12px;
  color: #666;
}

.result-message {
  margin-bottom: 10px;
  color: #374151;
}

.result-details {
  background: #f8fafc;
  padding: 10px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
}

.result-details pre {
  margin: 0;
  font-size: 12px;
  color: #475569;
  white-space: pre-wrap;
  word-break: break-all;
}

.tools-summary {
  display: flex;
  gap: 20px;
  margin-bottom: 15px;
}

.metric {
  display: flex;
  align-items: center;
  gap: 5px;
}

.label {
  color: #666;
  font-size: 14px;
}

.value {
  font-weight: 500;
}

.value.success {
  color: #22c55e;
}

.value.error {
  color: #ef4444;
}

.tools-list {
  display: grid;
  gap: 8px;
}

.tool-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: #f8fafc;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
}

.tool-name {
  font-weight: 500;
  color: #2c3e50;
}

.tool-category {
  background: #e0e7ff;
  color: #3730a3;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.tool-status {
  margin-left: auto;
}

.test-controls {
  display: flex;
  gap: 15px;
  align-items: end;
  margin-bottom: 15px;
  flex-wrap: wrap;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.input-group label {
  font-size: 14px;
  color: #374151;
  font-weight: 500;
}

.form-control {
  padding: 8px 12px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 14px;
}

.form-control:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.1);
}

.tool-result .metric {
  margin-bottom: 8px;
}

.tool-output, .tool-error {
  margin-top: 15px;
}

.tool-output h4, .tool-error h4 {
  margin-bottom: 8px;
  color: #374151;
  font-size: 14px;
}

.tool-output pre, .tool-error pre {
  background: #f8fafc;
  padding: 10px;
  border-radius: 6px;
  border: 1px solid #e2e8f0;
  font-size: 12px;
  color: #475569;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}

.error-message {
  background: #fef2f2;
  color: #dc2626;
  padding: 10px;
  border-radius: 6px;
  border: 1px solid #fecaca;
}

.summary-panel {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 20px;
  border-radius: 12px;
}

.summary-panel h3 {
  margin-bottom: 15px;
  color: white;
}

.summary-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 15px;
}

.metric-card {
  background: rgba(255, 255, 255, 0.1);
  padding: 15px;
  border-radius: 8px;
  text-align: center;
  backdrop-filter: blur(10px);
}

.metric-card.success {
  background: rgba(34, 197, 94, 0.2);
}

.metric-card.warning {
  background: rgba(245, 158, 11, 0.2);
}

.metric-card.error {
  background: rgba(239, 68, 68, 0.2);
}

.metric-title {
  font-size: 14px;
  opacity: 0.9;
  margin-bottom: 5px;
}

.metric-value {
  font-size: 24px;
  font-weight: bold;
}
</style>
