export default {
  // 页面标题和描述
  serversTitle: '工具管理',
  serversDescription: '管理内置工具、工作流、插件和MCP工具',
  expand: '展开',

  // 选项卡
  builtinTools: '内置工具',
  workflowTools: '工作流工具',
  pluginTools: '插件工具',
  mcpServers: 'MCP工具',
  marketplace: 'MCP市场',
  skills: 'Skills',
  skillsDescription: '管理本地 SKILL.md 与索引',
  skillsInstallFromGithub: '从 GitHub 安装',
  skillsImportFile: '导入文件',
  skillsImportFolder: '导入文件夹',
  skillsInstallHistory: '安装历史',
  skillsInstallHistoryEmpty: '暂无安装记录。',
  skillsInstallSelect: '选择要安装的 Skills',
  skillsInstallNoCandidates: '未在来源中发现 Skills。',
  skillsInstallConfirm: '安装',
  skillsGitUrl: 'Git URL',
  skillsGitUrlPlaceholder: 'https://github.com/owner/repo',
  skillsGitResolve: '解析并获取',
  skillsInstallSuccess: 'Skills 安装成功',
  skillsInstallFailed: 'Skills 安装失败',
  skillsImportFailed: 'Skills 导入失败',
  skillsInstallHistoryDeleteConfirm: '确定删除该安装记录？',
  skillsInstallHistoryDeleted: '安装记录已删除',
  skillsInstallHistoryDeleteFailed: '删除安装记录失败',
  skillsEnabledLabel: 'Skills 工具',
  skillsCardView: '卡片',
  skillsListView: '列表',
  skillsDisabledWarning: 'Skills 已禁用。请开启开关后再管理或使用 Skills。',
  skillsDropInstallHint: '拖放 skill 文件或文件夹到此处进行安装',
  skillsDropMultipleHint: '检测到多个项目，将使用第一个拖放路径进行安装。',
  skillsInstalledTitle: '已安装技能',
  skillsInstalledEmpty: '暂无已安装技能。',

  // 管理下拉菜单
  management: '管理',
  cleanupDuplicates: '清理重复服务器',

  // 编辑模式
  formEdit: '表单编辑',
  jsonEdit: 'JSON编辑',

  // 传输类型
  transportTypes: {
    stdio: '标准输入/输出 (stdio)',
    sse: '服务器发送事件 (sse)',
    streamableHttp: '可流式HTTP (streamableHttp)'
  },

  // JSON编辑警告
  jsonEditWarning: '直接编辑 JSON 配置，请确保格式正确',
  serverConfigJson: '服务器配置 (JSON)',

  // 测试服务器模态框
  testServerTitle: '测试服务器工具',
  loadingTools: '正在加载服务器工具列表...',
  selectToolInfo: '选择一个工具进行测试，可以使用默认参数或自定义参数。',
  selectTool: '选择工具',
  inputParamsDescription: '输入参数说明',
  paramName: '参数名',
  paramType: '类型',
  paramRequired: '必填',
  paramConstraints: '约束',
  required: '必填',
  testParams: '测试参数 (JSON，可选)',
  testParamsPlaceholder: '留空使用默认参数，或输入 JSON 对象覆盖默认参数',
  testResult: '测试结果',
  copyResult: '复制结果',
  toggleJsonView: '切换JSON渲染',
  copiedToClipboard: '已复制到剪贴板',
  copyFailed: '复制失败',
  jsonParseFailed: 'JSON解析失败',
  runTest: '运行测试',
  testing: '正在执行测试...',

  // 服务器详情模态框
  serverDetails: {
    title: '服务器详情',
    general: '常规',
    tools: '工具',
    paramName: '参数名称',
    paramType: '参数类型',
    paramRequired: '必需',
    paramConstraints: '约束',
    connectToViewTools: '请连接到服务器以查看其工具。',
    noTools: '此服务器不提供工具。',
    inputSchema: '输入模式',
    toolName: '工具名称',
    toolDesc: '描述',
    toolInput: '输入',
    toolOutput: '输出'
  },

  // 添加服务器模态框
  addServer: {
    title: '添加MCP工具',
    quickCreate: '快速创建',
    importFromJson: '从JSON导入',
    jsonPaste: '粘贴JSON配置',
    import: '导入',
    jsonRequired: '需要JSON配置',
    enabled: '已启用',
    command: '命令',
    args: '参数',
    params: '参数',
    paramsPlaceholder: '每行一个参数',
    envVars: '环境变量',
    timeout: '超时时间（秒）',
    importSuccess: '从JSON成功导入服务器！',
    importFailed: '从JSON导入服务器失败',
    added: '已添加'
  },

  // 消息
  updateSuccess: '服务器更新成功',
  updateFailed: '服务器更新失败',
  reconnectWarning: '但重新连接失败，请手动重连',
  reconnected: '，服务器已重新连接',
  importSuccess: '服务器导入成功',
  importFailed: '服务器导入失败',
  jsonFormatError: 'JSON 格式错误，请检查语法',
  serverNotConnected: '当前服务器未处于连接状态，无法测试工具',
  loadToolsFailed: '加载服务器工具列表失败',
  selectToolFirst: '请选择要测试的工具',
  paramsJsonError: '参数 JSON 格式错误，请检查',
  testingTool: '正在测试插件...',
  testCompleted: '工具测试完成',
  testFailed: '工具测试失败',
  cleanupConfirm: '确定要清理重复的MCP工具配置吗？这将删除重复的配置，只保留最新的。',
  cleanedDuplicates: '已清理 {count} 个重复配置',
  noDuplicates: '没有发现重复的服务器配置',
  cleanupFailed: '清理失败',

  // 旧版字段
  title: 'MCP工具',
  description: '管理和使用MCP工具和服务',
  mcpServer: 'MCP工具',
  endpoint: '端点',
  startServer: '启动服务器',
  stopServer: '停止服务器',
  availableTools: '可用工具',
  connections: '连接',
  mcpConnections: 'MCP连接',
  noConnections: '没有活动的MCP连接。',
  addConnection: '添加连接',
  searchPlaceholder: '搜索工具...',
  installFromGithub: '从GitHub安装',
  installFromGithubDescription: '从GitHub仓库URL安装MCP工具',
  githubUrl: 'GitHub URL',
  installFromFile: '从文件安装',
  installFromFileDescription: '从本地文件安装MCP工具',
  selectFile: '选择文件',
  installTypes: {
    url: 'URL',
    file: '文件',
    registry: '注册表',
    process: '子进程'
  },
  command: '命令',
  args: '参数',
  argsHint: '用空格分隔多个参数',
  commandHint: '请输入可执行文件的完整路径或确保命令在系统PATH中',
  commandNotFoundConfirm: '在系统中未找到命令。需要帮助吗？',
  commandNotFoundHelp: '请尝试以下解决方案：\\n1. 确保命令名称正确\\n2. 使用完整绝对路径（例如：C:\\\\Program Files\\\\app\\\\command.exe）\\n3. 将命令的目录添加到系统PATH环境变量\\n4. 如果是npm包，先全局安装（npm install -g package-name）',
  toolDescriptions: {
    fileSystem: '文件系统操作',
    textEditor: '文本编辑功能',
    codeAnalysis: '代码分析工具',
    webSearch: '网络搜索功能',
    database: '数据库操作',
    network: '网络工具',
    security: '安全测试工具',
    automation: '自动化实用程序'
  },

  // 插件管理
  plugins: {
    allStatus: '全部状态',
    noReviewPlugins: '没有需要审核的插件'
  },

  exploitdb: {
    title: 'ExploitDB',
    description: '管理 search_exploit 的数据源，并查看本地同步的漏洞利用条目。',
    openManager: '数据',
    configTitle: '数据源配置',
    configDescription: '配置本地 ExploitDB 仓库路径，并手动触发同步与索引刷新。',
    saveSettings: '保存配置',
    syncNow: '立即同步',
    refreshStatus: '刷新状态',
    statusTitle: '同步状态',
    repoUrl: '仓库地址',
    repoPath: '本地路径',
    selectPath: '选择路径',
    repoReady: '仓库就绪',
    indexReady: '索引就绪',
    indexedEntries: '索引条目数',
    lastCommit: '最近提交',
    lastSync: '最近同步',
    indexedAt: '索引时间',
    syncSuccess: 'ExploitDB 同步完成',
    browserTitle: '数据浏览',
    browserDescription: '支持按关键词、CVE、平台和类型检索本地 ExploitDB 索引。',
    totalResults: '共 {count} 条结果',
    repoNotReady: '本地 ExploitDB 仓库或索引尚未就绪，请先保存配置并执行同步。',
    emptySearch: '没有匹配的 ExploitDB 条目。',
    pageInfo: '第 {page} / {total} 页',
    detailTitle: '条目详情',
    detailEmpty: '从左侧选择一个条目查看详情。',
    openInToolTest: '在工具测试中打开',
    filters: {
      query: '关键词',
      queryPlaceholder: '产品名、组件、漏洞标题',
      cve: 'CVE',
      platform: '平台',
      platformPlaceholder: 'windows / linux / php',
      type: '类型',
      typePlaceholder: 'remote / webapps / dos'
    },
    detail: {
      author: '作者',
      publishedAt: '发布日期',
      platform: '平台',
      type: '类型',
      cves: '关联 CVE',
      pocCode: 'PoC / 利用代码',
      truncated: '内容已截断',
      copyPoc: '复制',
      copySuccess: 'PoC 已复制到剪贴板',
      copyFailed: '复制 PoC 失败'
    }
  },

  // Shell终端
  shell: {
    title: 'Shell 终端',
    clear: '清空',
    settings: '设置',
    executing: '执行中...',
    enterCommand: '输入命令...',
    welcome: 'Sentinel AI Shell 终端',
    welcomeHint: '输入命令并按 Enter 执行。输入 help 查看可用命令。',
    historyHint: '使用 ↑/↓ 浏览命令历史。',
    completedIn: '完成于 {time}ms (退出码: {code})',
    helpText: `可用命令:
  clear, cls    - 清空终端
  help          - 显示帮助信息
  cd <path>     - 切换目录
  pwd           - 显示当前目录
  exit          - 关闭终端
  
其他命令将在系统 Shell 中执行。`,
    openTerminal: '打开终端',
    securityConfig: '安全配置',
    defaultPolicy: '默认策略',
    defaultPolicyHint: '当命令不匹配任何规则时的处理方式',
    actionAllow: '允许执行',
    actionAsk: '询问用户',
    actionDeny: '拒绝执行',
    securityRules: '安全规则',
    addRule: '添加规则',
    ruleOrder: '顺序',
    rulePattern: '命令匹配模式 (包含匹配)',
    ruleAction: '动作',
    ruleOperations: '操作',
    noRules: '暂无规则，将使用默认策略',
    ruleHint: '规则按顺序匹配，一旦匹配成功即应用对应动作。建议将具体规则放在前面，通用规则放在后面。',
    saveConfig: '保存配置',
    configSaved: '配置已保存',
    configLoadFailed: '加载配置失败',
    configSaveFailed: '保存配置失败',
    patternRequired: '规则匹配模式不能为空',
    // Inline confirmation
    runCommand: '运行命令？',
    accept: '接受',
    reject: '拒绝',
    alwaysAccept: '始终接受',
    alwaysAcceptHint: '将此命令添加到允许列表并执行',
    allowRulePreviewTitle: '始终接受将保存以下规则：',
    semanticLabels: {
      read_only: '只读',
      readOnly: '只读',
      mutating: '变更',
      dangerous: '高风险'
    },
    semanticSummaries: {
      readOnly: '检测到只读命令。',
      mutating: '该命令可能会更改文件、进程或系统状态。',
      dangerous: '检测到高风险命令。'
    },
    semanticReasons: {
      readOnly: '该命令看起来只是在检查文件、Git 状态或系统信息，不会改变当前状态。',
      mutating: '该命令未被识别为只读操作，可能会修改文件、进程或系统状态。',
      dangerousPrivilegeEscalation: '该命令试图提升权限。',
      dangerousUserContextSwitch: '该命令试图切换用户上下文。'
    },
    allowRuleReasons: {
      keepOriginalReadOnly: '没有识别出更窄且安全的前缀，因此保留原始只读命令。',
      narrowReadOnlyBase: '规则收窄为只读基础命令，便于后续复用，同时避免放宽范围。',
      narrowTestCommand: '规则收窄为只读测试命令。',
      narrowCommandExists: '规则收窄为命令存在性检查。',
      narrowFind: '规则收窄为只读文件发现。',
      narrowGitInspection: '规则收窄为 Git 检查类命令。',
      narrowGitSubcommand: '规则收窄为具体的只读 Git 子命令。',
      narrowGitRemoteShow: '规则收窄为只读 Git 远端检查。',
      narrowGitRemoteGetUrl: '规则收窄为只读 Git 远端 URL 检查。',
      narrowGitStashList: '规则收窄为只读 stash 列表查看。',
      narrowGitStashShow: '规则收窄为只读 stash 内容检查。',
      exactMutatingSubcommand: '变更类命令保持精确子命令匹配，避免后续审批范围被放宽。'
    },
    success: '成功',
    failed: '失败',
    copyAll: '复制',
    copyAllHint: '复制命令与全部输出',
    outputTruncatedHint: '输出为性能考虑已截断，复制全部可查看完整输出。',
    noOutput: '（无输出）',
    longRunningCommandTitle: '该命令更适合在交互式终端中运行',
    longRunningCommandHint: '检测到这是一个后台或长驻命令。一次性 shell 会等待输出管道关闭，容易一直停留在执行中。请改用右侧 Terminal，或显式重定向 stdout/stderr 并彻底脱离终端。',
    clickToExpand: '点击展开',
    expand: '展开',
    collapse: '折叠'
  }
}
