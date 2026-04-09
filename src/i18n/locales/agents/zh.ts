export default {
  title: '智能体管理',
  agents: '系统智能体',
  activeAgents: '活跃系统智能体',
  inactiveAgents: '非活跃系统智能体',
  agentDetails: '智能体详情',
  agentName: '智能体名称',
  agentStatus: '智能体状态',
  agentType: '智能体类型',
  agentVersion: '智能体版本',
  lastSeen: '最后访问',
  searchPlaceholder: '搜索系统智能体...',
  noAgents: '未找到系统智能体',
  totalAgents: '系统智能体总数',

  stats: {
    totalScans: '总扫描数',
    successfulScans: '成功扫描数',
    failedScans: '失败扫描数',
    averageScanTime: '平均扫描时间',
    lastScanTime: '最后扫描时间',
  },

  table: {
    name: '名称',
    status: '状态',
    type: '类型',
    version: '版本',
    lastSeen: '最后访问',
    actions: '操作',
  },

  workflow: {
    title: '智能体工作流',
    createWorkflow: '创建工作流',
    editWorkflow: '编辑工作流',
    deleteWorkflow: '删除工作流',
    workflowName: '工作流名称',
    workflowDescription: '工作流描述',
    workflowSteps: '工作流步骤',
    searchPlaceholder: '搜索工作流...',
    noWorkflows: '未找到工作流',
    totalWorkflows: '工作流总数',

    form: {
      namePlaceholder: '输入工作流名称',
      descriptionPlaceholder: '输入工作流描述',
      selectType: '选择工作流类型',
    },
  },

  form: {
    namePlaceholder: '输入智能体名称',
    descriptionPlaceholder: '输入智能体描述',
    selectType: '选择智能体类型',
    selectStatus: '选择智能体状态',
  },

  taskTypes: {
    vulnerabilityScan: '漏洞扫描',
    penetrationTest: '渗透测试',
    reconnaissance: '侦察',
    exploitation: '利用',
    postExploitation: '后利用',
    reporting: '报告',
  },

  priority: {
    low: '低',
    medium: '中',
    high: '高',
    critical: '严重',
  },

  risk: {
    low: '低风险',
    medium: '中等风险',
    high: '高风险',
    critical: '严重风险',
  },
}
