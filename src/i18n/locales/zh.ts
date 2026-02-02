import common from './common/zh'
import sidebar from './sidebar/zh'
import dashboard from './dashboard/zh'
import settings from './settings/zh'
import ai from './settings/ai/zh'
import rag from './settings/rag/zh'
import database from './settings/database/zh'
import scanTasks from './scanTasks/zh'
import vulnerabilities from './vulnerabilities/zh'
import tools from './tools/zh'
import assetManagement from './assetManagement/zh'
import assetTypes from './assetTypes/zh'
import riskLevels from './riskLevels/zh'
import assetStatuses from './assetStatuses/zh'
import aiChat from './aiChat/zh'
import positions from './positions/zh'
import language from './language/zh'
import projects from './projects/zh'
import dictionary from './dictionary/zh'
import mcp from './mcp/zh'
import roles from './roles/zh'
import scanSessions from './scanSessions/zh'
import agents from './agents/zh'
import trafficAnalysis from './trafficAnalysis/zh'
import ragManagement from './rag/zh'
import notifications from './notifications/zh'
import aiAssistant from './aiAssistant/zh'
import agent from './agent/zh'
import bugBounty from './bugBounty/zh'

// Import other modules as needed
// For now, we'll include the basic structure and add more as we extract them

export default {
  common,
  sidebar,
  dashboard,
  settings: {
    ...settings,
    ai,
    rag,
    database,
  },
  scanTasks,
  vulnerabilities,
  tools,
  Tools: tools,
  assetManagement,
  assetTypes,
  riskLevels,
  assetStatuses,
  aiChat,
  positions,
  language,
  projects,
  dictionary,
  mcp,
  roles,
  scanSessions,
  agents,
  trafficAnalysis,
  ragManagement,
  notifications,
  aiAssistant,
  agent,
  bugBounty,

  // Top-level aliases for sidebar navigation
  rag: {
    title: '知识库管理'
  },
  workflow: {
    title: '工作流'
  },

  // Security Center section
  securityCenter: {
    title: '安全中心',
    tabs: {
      vulnerabilities: '漏洞',
      scanTasks: '扫描任务',
      assets: '资产',
    },
  },

  // Placeholder for remaining sections that need to be extracted
  // These will be added as we create the corresponding modules
  agentCreator: {
    title: 'Agent创建器',
    createAgent: '创建Agent',
    editAgent: '编辑Agent',
    deleteAgent: '删除Agent',
    agentName: 'Agent名称',
    agentDescription: 'Agent描述',
    agentType: 'Agent类型',
    agentCapabilities: 'Agent功能',
    searchPlaceholder: '搜索Agent...',
    noAgents: '未找到Agent',
    totalAgents: 'Agent总数',

    steps: {
      basic: '基本信息',
      capabilities: '功能',
      tools: '工具',
      code: '代码',
      deploy: '部署'
    },

    basic: {
      title: '基本信息',
      name: 'Agent名称',
      description: 'Agent描述',
      type: 'Agent类型',
      version: 'Agent版本',
      author: '作者',
      tags: '标签'
    },

    capabilities: {
      title: 'Agent功能',
      vulnerabilityScanning: '漏洞扫描',
      penetrationTesting: '渗透测试',
      reconnaissance: '侦察',
      exploitation: '利用',
      postExploitation: '后利用',
      reporting: '报告',
      automation: '自动化',
      analysis: '分析'
    },

    tools: {
      title: 'Agent工具',
      availableTools: '可用工具',
      selectedTools: '已选择工具',
      toolCategories: '工具分类',
      searchPlaceholder: '搜索工具...',
      noTools: '未找到工具',
      totalTools: '工具总数',

      categories: {
        network: '网络工具',
        web: 'Web工具',
        database: '数据库工具',
        system: '系统工具',
        exploitation: '利用工具',
        postExploitation: '后利用工具',
        reporting: '报告工具',
        utility: '实用工具'
      }
    },

    code: {
      title: 'Agent代码',
      generateCode: '生成代码',
      editCode: '编辑代码',
      saveCode: '保存代码',
      codeEditor: '代码编辑器',
      syntaxHighlighting: '语法高亮',
      autoComplete: '自动完成',
      errorChecking: '错误检查'
    },

    deploy: {
      title: '部署Agent',
      deployLocally: '本地部署',
      deployRemotely: '远程部署',
      deployToCloud: '部署到云端',
      deploymentStatus: '部署状态',
      deploymentProgress: '部署进度',
      deploymentSuccess: '部署成功',
      deploymentFailed: '部署失败'
    },

    navigation: {
      previous: '上一步',
      next: '下一步',
      finish: '完成',
      cancel: '取消'
    },

    messages: {
      agentCreated: 'Agent创建成功',
      agentUpdated: 'Agent更新成功',
      agentDeleted: 'Agent删除成功',
      agentDeployed: 'Agent部署成功',
      agentDeploymentFailed: 'Agent部署失败'
    }
  },

  // Plugins section
  plugins: {
    title: '插件管理',
    description: '管理和配置安全测试插件',
    plugins: '插件',
    installedPlugins: '已安装插件',
    availablePlugins: '可用插件',
    pluginDetails: '插件详情',
    pluginName: '插件名称',
    pluginDescription: '插件描述',
    pluginVersion: '插件版本',
    pluginAuthor: '插件作者',
    pluginStatus: '插件状态',
    searchPlaceholder: '搜索插件...',
    searchPlugins: '搜索插件...',
    noPlugins: '暂无插件，请上传或扫描插件目录',
    totalPlugins: '插件总数',
    allStatus: '全部',
    noReviewPlugins: '暂无待审核的插件',
    allPlugins: '全部插件',
    favorited: '已收藏',
    favorite: '收藏插件',
    unfavorite: '取消收藏',
    copyPlugin: '复制插件',
    showing: '显示',
    of: '共',
    items: '条',
    pageSize: '每页',

    review: {
      title: '插件审核',
      pendingReview: '待审核',
      pending: '待审核',
      approved: '已批准',
      rejected: '已拒绝',
      failed: '验证失败',
      reviewDetails: '审核详情',
      reviewComments: '审核评论',
      submitReview: '提交审核'
    },

    categories: {
      all: '全部',
      trafficAnalysis: '流量分析插件',
      agents: 'Agent工具插件',
      security: '安全',
      automation: '自动化',
      reporting: '报告',
      integration: '集成',
      utility: '实用程序',
      other: '其他'
    },
    mainCategory: '主分类',
    subCategory: '子分类',
    category: '分类',
    basicInfo: '基本信息',
    newPlugin: '新增插件',
    uploadPlugin: '上传插件',
    uploading: '上传中...',
    aiGenerate: 'AI生成插件',
    aiGenerating: 'AI正在生成插件代码，请稍候...',
    aiPrompt: '描述你想要的插件功能',
    aiPromptPlaceholder: '例如：我需要一个检测SQL注入漏洞的插件...',
    pluginReview: '插件审核',
    pluginDetail: '插件详情',
    pluginId: '插件ID',
    pluginIdPlaceholder: '例如: sql_injection_scanner',
    pluginNamePlaceholder: '例如: SQL注入扫描器',
    version: '版本',
    author: '作者',
    authorPlaceholder: '作者名称',
    pluginType: '插件类型',
    vulnType: '漏洞类型',
    severity: '严重程度',
    defaultSeverity: '默认严重程度',
    descriptionPlaceholder: '插件功能描述',
    tags: '标签',
    commaSeparated: '逗号分隔',
    tagsPlaceholder: '例如: security, scanner, sql',
    pluginCode: '插件代码',
    insertTemplate: '插入模板',
    format: '格式化',
    createPlugin: '创建插件',
    creating: '创建中...',
    confirmDelete: '确认删除',
    deleteConfirmText: '确定要删除插件',
    deleteWarning: '吗？此操作不可撤销。',
    deleting: '删除中...',
    upload: '上传',
    selectFile: '选择插件文件 (.ts / .js)',
    enable: '启用',
    disable: '禁用',
    enabled: '已启用',
    disabled: '已禁用',
    approved: '已批准',
    rejected: '已拒绝',
    pendingReview: '待审核',
    validationFailed: '验证失败',
    approve: '批准',
    reject: '拒绝',
    qualityScore: '质量评分',
    qualityBreakdown: '质量评分细分',
    model: '模型',
    generatedAt: '生成时间',
    syntaxScore: '语法正确性',
    logicScore: '逻辑完整性',
    securityScore: '安全性',
    codeQuality: '代码质量',
    validationResult: '验证结果',
    validationPassed: '验证通过',
    testResult: '插件测试结果',
    testPassed: '测试通过',
    testFailed: '测试失败',
    testMessage: '测试消息',
    testing: '正在测试...',
    startTest: '开始测试',
    advancedTest: '高级测试',
    requestUrl: '请求 URL',
    httpMethod: 'HTTP 方法',
    headersJson: '请求头 (JSON)',
    body: '请求体',
    runs: '运行次数',
    concurrency: '并发数',
    totalRuns: '总运行',
    totalDuration: '总耗时(ms)',
    avgPerRun: '平均/次(ms)',
    findingsTotal: '发现数',
    findings: '发现',
    unique: '唯一',
    runDetails: '运行详情',
    duration: '耗时(ms)',
    agentInputs: '插件入参 (JSON)',
    runOutput: '运行',
    executionResult: '执行结果',
    failed: '失败',
    success: '成功',
    agentToolResult: 'Agent工具执行结果',
    noOutputData: '无输出数据',
    codeEditor: '插件代码编辑器',
    readonly: '只读',
    cancelEdit: '取消编辑',
    exitFullscreen: '退出全屏',
    copy: '复制',
    copySuccess: '已复制',
    copyCode: '复制代码',
    copyFailed: '复制失败',
    collapse: '收起',
    expand: '展开',
    clearHistory: '清除历史',
    clearHistoryConfirm: '确定要清除这个插件的所有对话历史吗？',
    historyCleared: '对话历史已清除',
    codeHasErrors: '代码存在错误:\n{errors}\n\n是否仍要应用?',
    codeHasWarnings: '代码有警告: {warnings}',
    validating: '验证中...',
    explainCode: '解释代码',
    optimizeCode: '优化代码',
    fixBugs: '修复问题',
    refactorCode: '重构代码',
    securityCheck: '安全检查',
    addComments: '添加注释',
    generateTests: '生成测试',
    currentCode: '当前代码',
    aiSuggestion: 'AI 建议',
    aiAssistant: 'AI 助手',
    addToAiContext: '添加到 AI 上下文',
    addedToContext: '已添加到上下文',
    aiInputPlaceholder: '描述你想要的修改...',
    aiAssistantHint: '描述你想要的修改，AI 将帮助你编辑代码',
    fullCode: '完整代码',
    referToAi: '引用到AI助手',
    lines: '行',
    testResultRef: '测试结果',
    aiSuggestions: 'AI 修改建议',
    blocks: '个代码块',
    apply: '应用',
    preview: '预览',
    applyAll: '全部应用',
    shortcutHint: 'Enter发送 · Shift+Enter换行 · Ctrl+K切换面板',
    contextMenuHint: '右键编辑器添加代码到上下文',
    selectedLines: '选中代码',
    testSuccess: '测试成功',
    testError: '测试错误',
    pluginSaved: '插件已保存',
    pluginCreated: '插件创建成功',
    templateInserted: '已插入模板代码',
    usingBuiltinTemplate: '使用内置模板',
    codeFormatted: '代码已格式化',
    formatFailed: '格式化失败',
    codeApplied: '代码已应用',
    codeMerged: '代码已合并',
    reviewChanges: '请审查代码变更',
    fullscreenMinimized: '全屏编辑器已缩小',
    editorMinimized: '编辑器已缩小',
    shortcuts: '快捷键',
    keyboardShortcuts: '键盘快捷键',
    toggleAiPanel: '切换AI面板',
    savePlugin: '保存插件',
    formatCode: '格式化代码',
    toggleFullscreen: '切换全屏',
    enableEdit: '启用编辑',
    toggleSuccess: '插件已{action}: {name}',
    toggleFailed: '{action}失败: {error}',
    toggleError: '操作失败',
    favoritedSuccess: '已收藏',
    unfavoritedSuccess: '已取消收藏',
    favoriteError: '操作失败',
    loadReviewError: '加载审核插件失败',
    approveSuccess: '插件已批准: {name}',
    approveFailed: '批准失败: {error}',
    rejectSuccess: '插件已拒绝: {name}',
    rejectFailed: '拒绝失败: {error}',
    batchApprove: '批量批准',
    batchReject: '批量拒绝',
    batchApproveSuccess: '已批准 {count} 个插件',
    batchApproveFailed: '批量批准失败: {error}',
    batchRejectSuccess: '已拒绝 {count} 个插件',
    batchRejectFailed: '批量拒绝失败: {error}',
    generatePlugin: '生成插件',
    warnings: '警告',
    errors: '错误',
    error: '错误',
    store: {
      title: '插件商店',
      searchPlaceholder: '搜索插件...',
      allCategories: '全部分类',
      refreshToLoad: '点击刷新获取插件列表',
      noPlugins: '暂无可用插件',
      noDescription: '暂无描述',
      installed: '已安装',
      install: '安装',
      installSuccess: '插件安装成功',
      installError: '安装失败',
      fetchError: '获取插件列表失败',
      downloading: '下载中...',
      viewDetails: '查看详情',
      justNow: '刚刚更新',
      minutesAgo: '{minutes}分钟前',
      hoursAgo: '{hours}小时前',
      listView: '列表视图',
      cardView: '卡片视图'
    },

    // Agent plugin subcategories
    agentCategories: {
      recon: '信息收集',
      discovery: '目标发现',
      vuln: '漏洞扫描',
      exploit: '漏洞利用',
      monitor: '变更监控',
      utility: '实用工具',
      scanner: '扫描工具',
      analyzer: '分析工具',
      reporter: '报告工具',
      custom: '自定义'
    }
  },

  // License section
  license: {
    title: '许可证管理',
    licenseStatus: '许可证状态',
    licenseType: '许可证类型',
    licenseKey: '许可证密钥',
    licenseExpiry: '许可证到期',
    licenseFeatures: '许可证功能',
    activateLicense: '激活许可证',
    deactivateLicense: '停用许可证',
    renewLicense: '续订许可证',
    upgradeLicense: '升级许可证',

    status: {
      active: '活跃',
      inactive: '非活跃',
      expired: '已过期',
      trial: '试用',
      invalid: '无效'
    },

    types: {
      trial: '试用',
      standard: '标准',
      professional: '专业',
      enterprise: '企业'
    }
  },

  // Proxifier Panel section
  proxifierPanel: {
    rules: {
      title: '代理规则',
      noRules: '暂无规则',
      table: {
        name: '名称',
        applications: '应用',
        applicationsExample: '示例：Chrome、Safari',
        targetHosts: '目标主机',
        targetHostsExample: '示例：*.example.com',
        targetPorts: '目标端口',
        targetPortsExample: '示例：80、443',
        action: '动作',
        direct: '直连',
        block: '阻止',
        viaProxy: '通过代理',
        proxyFormat: '代理: [代理名称]'
      }
    },
    buttons: {
      add: '添加',
      clone: '克隆',
      edit: '编辑',
      remove: '删除',
      enabled: '已启用',
      cancel: '取消',
      save: '保存',
      close: '关闭'
    }
  }
}
