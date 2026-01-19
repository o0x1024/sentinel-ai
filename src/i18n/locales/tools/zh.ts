export default {
  // 页面标题和描述
  serversTitle: '工具管理',
  serversDescription: '管理内置工具、工作流、插件和MCP服务器',

  // 选项卡
  builtinTools: '内置工具',
  workflowTools: '工作流工具',
  pluginTools: '插件工具',
  mcpServers: 'MCP服务器',
  marketplace: 'MCP市场',

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
    title: '添加MCP服务器',
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
  cleanupConfirm: '确定要清理重复的MCP服务器配置吗？这将删除重复的配置，只保留最新的。',
  cleanedDuplicates: '已清理 {count} 个重复配置',
  noDuplicates: '没有发现重复的服务器配置',
  cleanupFailed: '清理失败',

  // 旧版字段
  title: 'MCP工具',
  description: '管理和使用MCP工具和服务',
  mcpServer: 'MCP服务器',
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
    success: '成功',
    failed: '失败',
    copyAll: '复制全部',
    copyAllHint: '复制命令与全部输出',
    outputTruncatedHint: '输出为性能考虑已截断，复制全部可查看完整输出。',
    noOutput: '（无输出）',
    clickToExpand: '点击展开',
    expand: '展开',
    collapse: '折叠'
  }
}
