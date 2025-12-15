export default {
  // Page title and description
  serversTitle: 'Tool Management',
  serversDescription: 'Manage built-in tools, workflows, plugins, and MCP servers',

  // Tabs
  builtinTools: 'Built-in Tools',
  workflowTools: 'Workflow Tools',
  pluginTools: 'Plugin Tools',
  mcpServers: 'MCP Servers',
  marketplace: 'Marketplace',

  // Management dropdown
  management: 'Management',
  cleanupDuplicates: 'Cleanup Duplicate Servers',

  // Edit modes
  formEdit: 'Form Edit',
  jsonEdit: 'JSON Edit',

  // Transport types
  transportTypes: {
    stdio: 'Standard Input/Output (stdio)',
    sse: 'Server-Sent Events (sse)',
    streamableHttp: 'Streamable HTTP (streamableHttp)'
  },

  // JSON edit warning
  jsonEditWarning: 'Directly edit JSON configuration, please ensure the format is correct',
  serverConfigJson: 'Server Configuration (JSON)',

  // Test server modal
  testServerTitle: 'Test Server Tool',
  loadingTools: 'Loading server tool list...',
  selectToolInfo: 'Select a tool to test, you can use default parameters or custom parameters.',
  selectTool: 'Select Tool',
  inputParamsDescription: 'Input Parameters Description',
  paramName: 'Parameter Name',
  paramType: 'Type',
  paramRequired: 'Required',
  paramConstraints: 'Constraints',
  required: 'Required',
  testParams: 'Test Parameters (JSON, optional)',
  testParamsPlaceholder: 'Leave empty to use default parameters, or enter JSON object to override default parameters',
  testResult: 'Test Result',
  runTest: 'Run Test',
  testing: 'Testing...',

  // Server details modal
  serverDetails: {
    title: 'Server Details',
    general: 'General',
    tools: 'Tools',
    paramName: 'Param Name',
    paramType: 'Param Type',
    paramRequired: 'Required',
    paramConstraints: 'Constraints',
    connectToViewTools: 'Please connect to the server to view its tools.',
    noTools: 'This server provides no tools.',
    inputSchema: 'Input Schema',
    toolName: 'Tool Name',
    toolDesc: 'Description',
    toolInput: 'Input',
    toolOutput: 'Output'
  },

  // Add server modal
  addServer: {
    title: 'Add MCP Server',
    quickCreate: 'Quick Create',
    importFromJson: 'Import from JSON',
    jsonPaste: 'Paste JSON Configuration',
    import: 'Import',
    jsonRequired: 'JSON configuration is required',
    enabled: 'Enabled',
    command: 'Command',
    args: 'Arguments',
    params: 'Parameters',
    paramsPlaceholder: 'One parameter per line',
    envVars: 'Environment Variables',
    timeout: 'Timeout (seconds)',
    importSuccess: 'Successfully imported server from JSON!',
    importFailed: 'Failed to import server from JSON',
    added: 'Added'
  },

  // Messages
  updateSuccess: 'Server updated successfully',
  updateFailed: 'Failed to update server',
  reconnectWarning: 'but reconnection failed, please reconnect manually',
  reconnected: ', server has been reconnected',
  importSuccess: 'Servers imported successfully',
  importFailed: 'Failed to import servers',
  jsonFormatError: 'JSON format error, please check syntax',
  serverNotConnected: 'Current server is not connected, cannot test tools',
  loadToolsFailed: 'Failed to load server tool list',
  selectToolFirst: 'Please select a tool to test',
  paramsJsonError: 'Parameters JSON format error, please check',
  testingTool: 'Testing tool...',
  testCompleted: 'Tool test completed',
  testFailed: 'Tool test failed',
  cleanupConfirm: 'Are you sure you want to cleanup duplicate MCP server configurations? This will delete duplicate configurations and only keep the latest one.',
  cleanedDuplicates: 'Cleaned {count} duplicate configurations',
  noDuplicates: 'No duplicate server configurations found',
  cleanupFailed: 'Cleanup failed',

  // Plugin management
  plugins: {
    allStatus: 'All Status',
    noReviewPlugins: 'No plugins require review'
  },

  // Legacy fields
  title: 'MCP Tools',
  description: 'Manage and use MCP tools and services',
  mcpServer: 'MCP Server',
  endpoint: 'Endpoint',
  startServer: 'Start Server',
  stopServer: 'Stop Server',
  availableTools: 'Available Tools',
  connections: 'Connections',
  mcpConnections: 'MCP Connections',
  noConnections: 'No active MCP connections.',
  addConnection: 'Add Connection',
  searchPlaceholder: 'Search for tools...',
  installFromGithub: 'Install from GitHub',
  installFromGithubDescription: 'Install MCP tool from a GitHub repository URL',
  githubUrl: 'GitHub URL',
  installFromFile: 'Install from File',
  installFromFileDescription: 'Install MCP tool from a local file',
  selectFile: 'Select File',
  installTypes: {
    url: 'URL',
    file: 'File',
    registry: 'Registry',
    process: 'Sub-process'
  },
  command: 'Command',
  args: 'Arguments',
  argsHint: 'Separate multiple arguments with spaces',
  commandHint: 'Please enter the full path of the executable or ensure the command is in the system PATH',
  commandNotFoundConfirm: 'Command not found in the system. Do you need help?',
  commandNotFoundHelp: 'Please try the following solutions:\n1. Ensure the command name is correct\n2. Use the full absolute path (e.g., C:\\Program Files\\app\\command.exe)\n3. Add the command\'s directory to the system PATH environment variable\n4. If it\'s an npm package, install it globally first (npm install -g package-name)',
  toolDescriptions: {
    fileSystem: 'File system operations',
    textEditor: 'Text editing capabilities',
    codeAnalysis: 'Code analysis tools',
    webSearch: 'Web search functionality',
    database: 'Database operations',
    network: 'Network tools',
    security: 'Security testing tools',
    automation: 'Automation utilities'
  }
}
