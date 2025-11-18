import { computed, ref } from 'vue'

export interface OrchestratorSession {
  sessionId: string
  taskKind: string
  primaryTarget: string
  stage: string
  summary: string
  totalSteps: number
  totalFindings: number
  highRiskFindings: number
}

export interface OrchestratorStep {
  stepId: string
  index: number
  subAgentKind: 'ReWOO' | 'PlanAndExecute' | 'LLMCompiler' | 'Other'
  stepType: string
  shortSummary: string
  riskImpact: 'None' | 'Info' | 'Low' | 'Medium' | 'High' | 'Critical'
  status: 'pending' | 'running' | 'completed' | 'failed'
  output?: string
}

export function useOrchestratorMessage(content: string) {
  const parsedContent = ref<any>(null)
  const isOrchestratorMessage = ref(false)
  const messageType = ref<'session' | 'step' | null>(null)

  try {
    const parsed = JSON.parse(content)
    parsedContent.value = parsed

    if (parsed.type === 'orchestrator_session') {
      isOrchestratorMessage.value = true
      messageType.value = 'session'
    } else if (parsed.type === 'orchestrator_step') {
      isOrchestratorMessage.value = true
      messageType.value = 'step'
    }
  } catch (e) {
    // Not a JSON message or not an orchestrator message
    isOrchestratorMessage.value = false
  }

  const sessionData = computed<OrchestratorSession | null>(() => {
    if (messageType.value === 'session' && parsedContent.value) {
      return {
        sessionId: parsedContent.value.session_id,
        taskKind: parsedContent.value.task_kind,
        primaryTarget: parsedContent.value.primary_target,
        stage: parsedContent.value.stage,
        summary: parsedContent.value.summary,
        totalSteps: parsedContent.value.total_steps,
        totalFindings: parsedContent.value.total_findings,
        highRiskFindings: parsedContent.value.high_risk_findings,
      }
    }
    return null
  })

  const stepData = computed<OrchestratorStep | null>(() => {
    if (messageType.value === 'step' && parsedContent.value) {
      return {
        stepId: parsedContent.value.step_id,
        index: parsedContent.value.index,
        subAgentKind: parsedContent.value.sub_agent_kind,
        stepType: parsedContent.value.step_type,
        shortSummary: parsedContent.value.short_summary,
        riskImpact: parsedContent.value.risk_impact,
        status: parsedContent.value.status,
        output: parsedContent.value.output,
      }
    }
    return null
  })

  const taskKindLabel = computed(() => {
    const kind = sessionData.value?.taskKind
    const labels: Record<string, string> = {
      'web_pentest': 'Web 渗透测试',
      'api_pentest': 'API 渗透测试',
      'forensics': '取证分析',
      'ctf': 'CTF 解题',
      'reverse_engineering': '逆向工程',
      'other_security': '其他安全任务',
    }
    return kind ? labels[kind] || kind : ''
  })

  const stageLabel = computed(() => {
    const stage = sessionData.value?.stage
    const labels: Record<string, string> = {
      'recon': '信息收集',
      'login': '登录测试',
      'api_mapping': 'API 枚举',
      'vuln_scan': '漏洞扫描',
      'exploit': '漏洞利用',
      'log_collection': '日志收集',
      'timeline_reconstruction': '时间线重建',
      'ioc_extraction': 'IOC 提取',
      'behavior_analysis': '行为分析',
      'challenge_analysis': '题目分析',
      'vuln_identification': '漏洞识别',
      'payload_crafting': 'Payload 构造',
      'flag_extraction': 'Flag 提取',
      'writeup': 'Writeup',
      'binary_loading': '二进制加载',
      'static_analysis': '静态分析',
      'dynamic_analysis': '动态分析',
      'deobfuscation': '反混淆',
      'behavior_summary': '行为总结',
      'report': '报告生成',
      'completed': '已完成',
    }
    return stage ? labels[stage] || stage : ''
  })

  const subAgentLabel = computed(() => {
    const kind = stepData.value?.subAgentKind
    const labels: Record<string, string> = {
      'ReWOO': 'ReWOO 规划',
      'PlanAndExecute': '执行引擎',
      'LLMCompiler': '代码生成',
      'Other': '其他',
    }
    return kind ? labels[kind] || kind : ''
  })

  const riskColor = computed(() => {
    const risk = stepData.value?.riskImpact
    const colors: Record<string, string> = {
      'None': 'text-gray-500',
      'Info': 'text-blue-500',
      'Low': 'text-green-500',
      'Medium': 'text-yellow-500',
      'High': 'text-orange-500',
      'Critical': 'text-red-600',
    }
    return risk ? colors[risk] || 'text-gray-500' : 'text-gray-500'
  })

  const statusColor = computed(() => {
    const status = stepData.value?.status
    const colors: Record<string, string> = {
      'pending': 'text-gray-400',
      'running': 'text-blue-500',
      'completed': 'text-green-500',
      'failed': 'text-red-500',
    }
    return status ? colors[status] || 'text-gray-400' : 'text-gray-400'
  })

  const statusIcon = computed(() => {
    const status = stepData.value?.status
    const icons: Record<string, string> = {
      'pending': '⏳',
      'running': '▶️',
      'completed': '✅',
      'failed': '❌',
    }
    return status ? icons[status] || '•' : '•'
  })

  return {
    isOrchestratorMessage,
    messageType,
    sessionData,
    stepData,
    taskKindLabel,
    stageLabel,
    subAgentLabel,
    riskColor,
    statusColor,
    statusIcon,
  }
}

