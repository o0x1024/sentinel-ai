import common from '../common/en'
import sidebar from '../sidebar/en'
import dashboard from '../dashboard/en'
import settings from '../settings/en'
import ai from '../settings/ai/en'
import rag from '../settings/rag/en'
import database from '../settings/database/en'
import scanTasks from '../scanTasks/en'
import vulnerabilities from '../vulnerabilities/en'
import tools from '../tools/en'
import assetManagement from '../assetManagement/en'
import assetTypes from '../assetTypes/en'
import riskLevels from '../riskLevels/en'
import assetStatuses from '../assetStatuses/en'
import aiChat from '../aiChat/en'
import positions from '../positions/en'
import language from '../language/en'
import projects from '../projects/en'
import dictionary from '../dictionary/en'
import mcp from '../mcp/en'
import roles from '../roles/en'
import scanSessions from '../scanSessions/en'
import agents from '../agents/en'
import trafficAnalysis from '../trafficAnalysis/en'
import ragManagement from '../rag/en'
import notifications from '../notifications/en'
import aiAssistant from '../aiAssistant/en'
import agent from '../agent/en'
import bugBounty from '../bugBounty/en'

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
    title: 'RAG Management'
  },
  workflow: {
    title: 'Workflow Studio'
  },

  // Placeholder for remaining sections that need to be extracted
  // These will be added as we create the corresponding modules
  agentCreator: {
    title: 'Agent Creator',
    createAgent: 'Create Agent',
    editAgent: 'Edit Agent',
    deleteAgent: 'Delete Agent',
    agentName: 'Agent Name',
    agentDescription: 'Agent Description',
    agentType: 'Agent Type',
    agentCapabilities: 'Agent Capabilities',
    searchPlaceholder: 'Search agents...',
    noAgents: 'No agents found',
    totalAgents: 'Total Agents',

    steps: {
      basic: 'Basic Information',
      capabilities: 'Capabilities',
      tools: 'Tools',
      code: 'Code',
      deploy: 'Deploy'
    },

    basic: {
      title: 'Basic Information',
      name: 'Agent Name',
      description: 'Agent Description',
      type: 'Agent Type',
      version: 'Agent Version',
      author: 'Author',
      tags: 'Tags'
    },

    capabilities: {
      title: 'Agent Capabilities',
      vulnerabilityScanning: 'Vulnerability Scanning',
      penetrationTesting: 'Penetration Testing',
      reconnaissance: 'Reconnaissance',
      exploitation: 'Exploitation',
      postExploitation: 'Post-Exploitation',
      reporting: 'Reporting',
      automation: 'Automation',
      analysis: 'Analysis'
    },

    tools: {
      title: 'Agent Tools',
      availableTools: 'Available Tools',
      selectedTools: 'Selected Tools',
      toolCategories: 'Tool Categories',
      searchPlaceholder: 'Search tools...',
      noTools: 'No tools found',
      totalTools: 'Total Tools',

      categories: {
        network: 'Network Tools',
        web: 'Web Tools',
        database: 'Database Tools',
        system: 'System Tools',
        exploitation: 'Exploitation Tools',
        postExploitation: 'Post-Exploitation Tools',
        reporting: 'Reporting Tools',
        utility: 'Utility Tools'
      }
    },

    code: {
      title: 'Agent Code',
      generateCode: 'Generate Code',
      editCode: 'Edit Code',
      saveCode: 'Save Code',
      codeEditor: 'Code Editor',
      syntaxHighlighting: 'Syntax Highlighting',
      autoComplete: 'Auto Complete',
      errorChecking: 'Error Checking'
    },

    deploy: {
      title: 'Deploy Agent',
      deployLocally: 'Deploy Locally',
      deployRemotely: 'Deploy Remotely',
      deployToCloud: 'Deploy to Cloud',
      deploymentStatus: 'Deployment Status',
      deploymentProgress: 'Deployment Progress',
      deploymentSuccess: 'Deployment Success',
      deploymentFailed: 'Deployment Failed'
    },

    navigation: {
      previous: 'Previous',
      next: 'Next',
      finish: 'Finish',
      cancel: 'Cancel'
    },

    messages: {
      agentCreated: 'Agent created successfully',
      agentUpdated: 'Agent updated successfully',
      agentDeleted: 'Agent deleted successfully',
      agentDeployed: 'Agent deployed successfully',
      agentDeploymentFailed: 'Agent deployment failed'
    }
  },

  // Plugins section
  plugins: {
    title: 'Plugin Management',
    description: 'Manage and configure security testing plugins',
    plugins: 'Plugins',
    installedPlugins: 'Installed Plugins',
    availablePlugins: 'Available Plugins',
    pluginDetails: 'Plugin Details',
    pluginName: 'Plugin Name',
    pluginDescription: 'Plugin Description',
    pluginVersion: 'Plugin Version',
    pluginAuthor: 'Plugin Author',
    pluginStatus: 'Plugin Status',
    searchPlaceholder: 'Search plugins...',
    noPlugins: 'No plugins found',
    totalPlugins: 'Total Plugins',

    // New translations
    mainCategory: 'Main Category',
    subCategory: 'Sub Category',
    newPlugin: 'New Plugin',
    uploadPlugin: 'Upload Plugin',
    aiGenerate: 'AI Generate',
    pluginReview: 'Plugin Review',
    searchPlugins: 'Search plugins',
    batchApprove: 'Batch Approve',
    batchReject: 'Batch Reject',
    qualityScore: 'Quality Score',
    model: 'Model',
    generatedAt: 'Generated At',
    pendingReview: 'Pending Review',
    approved: 'Approved',
    rejected: 'Rejected',
    validationFailed: 'Validation Failed',
    pluginDetail: 'Plugin Detail',
    approve: 'Approve',
    reject: 'Reject',
    selectFile: 'Select File',
    upload: 'Upload',
    pluginId: 'Plugin ID',
    pluginIdPlaceholder: 'Enter plugin ID',
    pluginNamePlaceholder: 'Enter plugin name',
    version: 'Version',
    author: 'Author',
    authorPlaceholder: 'Enter author name',
    defaultSeverity: 'Default Severity',
    descriptionPlaceholder: 'Enter plugin description',
    tags: 'Tags',
    commaSeparated: 'Comma separated',
    tagsPlaceholder: 'Enter tags, separated by commas',
    pluginCode: 'Plugin Code',
    insertTemplate: 'Insert Template',
    format: 'Format',
    codePlaceholder: 'Enter plugin code',
    createPlugin: 'Create Plugin',
    confirmDelete: 'Confirm Delete',
    deleteConfirmText: 'Confirm delete plugin',
    deleteWarning: 'Delete Warning',
    aiPrompt: 'AI Prompt',
    aiPromptPlaceholder: 'Enter AI generation prompt',
    pluginType: 'Plugin Type',
    severity: 'Severity',
    generatePlugin: 'Generate Plugin',
    testResult: 'Test Result',
    advancedTest: 'Advanced Test',
    requestUrl: 'Request URL',
    httpMethod: 'HTTP Method',
    runs: 'Runs',
    concurrency: 'Concurrency',
    headersJson: 'Headers (JSON)',
    headersHint: 'Request headers in JSON format',
    body: 'Request Body',
    startTest: 'Start Test',
    error: 'Error',
    test: 'Test',
    code: 'Code',
    enabled: 'Enabled',
    disabled: 'Disabled',
    enable: 'Enable',
    disable: 'Disable',
    category: 'Category',
    basicInfo: 'Basic Info',
    vulnType: 'Vulnerability Type',
    qualityBreakdown: 'Quality Breakdown',
    syntaxScore: 'Syntax Score',
    securityScore: 'Security Score',
    codeQuality: 'Code Quality',
    validationResult: 'Validation Result',
    codeEditor: 'Code Editor',
    showing: 'Showing',
    of: 'of',
    items: 'items',
    pageSize: 'Page Size',
    favorited: 'Favorited',
    favorite: 'Favorite Plugin',
    unfavorite: 'Unfavorite',
    copyPlugin: 'Copy Plugin',
    allPlugins: 'All Plugins',
    agentInputs: 'Agent Inputs (JSON)',
    allStatus: 'All',
    noReviewPlugins: 'No plugins pending review',
    aiGenerating: 'AI is generating plugin code, please wait...',
    creating: 'Creating...',
    deleting: 'Deleting...',
    uploading: 'Uploading...',
    readonly: 'Read-only',
    copy: 'Copy',
    copySuccess: 'Copied',
    cancelEdit: 'Cancel Edit',
    exitFullscreen: 'Exit Fullscreen',
    approveSuccess: 'Plugin approved: {name}',
    approveFailed: 'Approve failed: {error}',
    rejectSuccess: 'Plugin rejected: {name}',
    rejectFailed: 'Reject failed: {error}',
    batchApproveSuccess: 'Approved {count} plugins',
    batchApproveFailed: 'Batch approve failed: {error}',
    batchRejectSuccess: 'Rejected {count} plugins',
    batchRejectFailed: 'Batch reject failed: {error}',
    toggleSuccess: 'Plugin {action}: {name}',
    toggleFailed: '{action} failed: {error}',
    toggleError: 'Operation failed',
    favoriteError: 'Operation failed',
    favoritedSuccess: 'Added to favorites',
    unfavoritedSuccess: 'Removed from favorites',
    validationPassed: 'Validation Passed',
    testPassed: 'Test Passed',
    testFailed: 'Test Failed',
    testMessage: 'Test Message',
    warnings: 'Warnings',
    errors: 'Errors',
    findings: 'Findings',
    findingsTotal: 'Total Findings',
    duration: 'Duration (ms)',
    totalRuns: 'Total Runs',
    totalDuration: 'Total Duration (ms)',
    avgPerRun: 'Avg/Run (ms)',
    runDetails: 'Run Details',
    unique: 'Unique',
    runOutput: 'Run',
    executionResult: 'Execution Result',
    failed: 'Failed',
    success: 'Success',
    agentToolResult: 'Agent Tool Execution Result',
    noOutputData: 'No output data',
    logicScore: 'Logic Score',
    loadReviewError: 'Failed to load review plugins',
    testing: 'Testing...',

    review: {
      title: 'Plugin Review',
      pendingReview: 'Pending Review',
      pending: 'Pending',
      approved: 'Approved',
      rejected: 'Rejected',
      failed: 'Validation Failed',
      reviewDetails: 'Review Details',
      reviewComments: 'Review Comments',
      submitReview: 'Submit Review'
    },

    categories: {
      all: 'All',
      trafficAnalysis: 'Traffic Analysis Plugins',
      agents: 'Agent Tool Plugins',
      builtinTools: 'Built-in Tool Plugins',
      mcpTools: 'MCP Tool Plugins',
      security: 'Security',
      automation: 'Automation',
      reporting: 'Reporting',
      integration: 'Integration',
      utility: 'Utility',
      vulnerability: 'Vulnerability Detection',
      injection: 'Injection Detection',
      xss: 'Cross-Site Scripting',
      scanner: 'Scanner',
      analyzer: 'Analyzer',
      reporter: 'Report Generator',
      custom: 'Custom',
      other: 'Other'
    },

    store: {
      title: 'Plugin Store',
      searchPlaceholder: 'Search plugins...',
      allCategories: 'All Categories',
      refreshToLoad: 'Click refresh to load plugins',
      noPlugins: 'No plugins available',
      noDescription: 'No description',
      installed: 'Installed',
      install: 'Install',
      installSuccess: 'Plugin installed successfully',
      installError: 'Installation failed',
      fetchError: 'Failed to fetch plugin list',
      downloading: 'Downloading...',
      viewDetails: 'View Details',
      justNow: 'Just updated',
      minutesAgo: '{minutes} minutes ago',
      hoursAgo: '{hours} hours ago',
      listView: 'List View',
      cardView: 'Card View'
    }
  },

  // Security Center section
  securityCenter: {
    title: 'Security Center',
    tabs: {
      vulnerabilities: 'Vulnerabilities',
      scanTasks: 'Scan Tasks',
      assets: 'Assets'
    }
  },

  // License section
  license: {
    title: 'License Management',
    licenseStatus: 'License Status',
    licenseType: 'License Type',
    licenseKey: 'License Key',
    licenseExpiry: 'License Expiry',
    licenseFeatures: 'License Features',
    activateLicense: 'Activate License',
    deactivateLicense: 'Deactivate License',
    renewLicense: 'Renew License',
    upgradeLicense: 'Upgrade License',

    status: {
      active: 'Active',
      inactive: 'Inactive',
      expired: 'Expired',
      trial: 'Trial',
      invalid: 'Invalid'
    },

    types: {
      trial: 'Trial',
      standard: 'Standard',
      professional: 'Professional',
      enterprise: 'Enterprise'
    }
  },

  // Proxifier Panel section
  proxifierPanel: {
    rules: {
      title: 'Proxy Rules',
      noRules: 'No rules',
      table: {
        name: 'Name',
        applications: 'Applications',
        applicationsExample: 'Example: Chrome, Safari',
        targetHosts: 'Target Hosts',
        targetHostsExample: 'Example: *.example.com',
        targetPorts: 'Target Ports',
        targetPortsExample: 'Example: 80, 443',
        action: 'Action',
        direct: 'Direct',
        block: 'Block',
        viaProxy: 'Via Proxy',
        proxyFormat: 'Proxy: [Proxy Name]'
      }
    },
    buttons: {
      add: 'Add',
      clone: 'Clone',
      edit: 'Edit',
      remove: 'Remove',
      enabled: 'Enabled',
      cancel: 'Cancel',
      save: 'Save',
      close: 'Close'
    }
  }
}
