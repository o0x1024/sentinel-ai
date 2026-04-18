export interface HelpCenterStat {
  label: string
  value: string
}

export interface HelpCenterWorkflowStep {
  title: string
  description: string
}

export interface HelpCenterModule {
  icon: string
  title: string
  route: string
  summary: string
  scenarios: string[]
}

export interface HelpCenterShortcut {
  keys: string
  description: string
}

export interface HelpCenterFaq {
  question: string
  answer: string
}

export interface HelpCenterContent {
  badge: string
  title: string
  description: string
  releaseNote: string
  stats: HelpCenterStat[]
  workflowTitle: string
  workflowDescription: string
  workflowSteps: HelpCenterWorkflowStep[]
  modulesTitle: string
  modulesDescription: string
  modules: HelpCenterModule[]
  shortcutsTitle: string
  shortcutsDescription: string
  shortcuts: HelpCenterShortcut[]
  faqTitle: string
  faqDescription: string
  faqs: HelpCenterFaq[]
  closingNote: string
}

const zhContent: HelpCenterContent = {
  badge: 'Sentinel AI 内置帮助中心',
  title: '把常用能力串成一条可执行的安全工作流',
  description:
    '这份帮助文档只说明当前版本里真实存在的入口和职责，目标不是堆概念，而是帮你更快判断“现在该去哪个模块做什么”。',
  releaseNote: '文档内容随当前桌面端版本一起发布，可离线查看。',
  stats: [
    { label: '入口组织', value: '导航栏 + 侧边栏' },
    { label: '推荐主线', value: '采集 -> 分析 -> 复现 -> 固化' },
    { label: '协作方式', value: 'AI、工作流、插件联动' },
  ],
  workflowTitle: '推荐使用路径',
  workflowDescription: '从第一性原理看，安全平台的核心是先拿到可靠输入，再把输入变成可复现结论。',
  workflowSteps: [
    {
      title: '1. 在流量分析中拿到原始事实',
      description:
        '优先采集请求、响应、证书、上下文和可复现样本。没有高质量输入，后续所有判断都会漂。',
    },
    {
      title: '2. 在安全中心完成筛选与定级',
      description: '把噪音和真实风险拆开，集中处理漏洞、扫描任务、资产与工作台中的上下文。',
    },
    {
      title: '3. 用爆破器、重放器、比较器复现假设',
      description: '把“可能有问题”收敛成“可以稳定复现的问题”，这是交付质量的分水岭。',
    },
    {
      title: '4. 用 AI、知识库、工作流把经验沉淀下来',
      description:
        '能模板化的流程交给工作流，能复用的知识放进 RAG，能自动检测的逻辑固化到插件或工具里。',
    },
  ],
  modulesTitle: '核心模块说明',
  modulesDescription: '每个模块都只解决一类问题。先分清职责，再决定入口，效率会高很多。',
  modules: [
    {
      icon: 'fas fa-chart-line',
      title: '总览 / Dashboard',
      route: '/dashboard',
      summary: '看系统整体状态、任务密度和各模块的健康度，适合做全局判断。',
      scenarios: ['刚进入系统时先看总体态势', '确认最近扫描、插件、知识库是否有异常'],
    },
    {
      icon: 'fas fa-network-wired',
      title: '流量分析 / Traffic',
      route: '/traffic',
      summary: '采集、筛选、重放和爆破 HTTP 流量，是定位问题最靠近事实的一层。',
      scenarios: ['做代理抓包、请求重放、响应比较', '验证参数污染、鉴权绕过、输入处理缺陷'],
    },
    {
      icon: 'fas fa-shield-virus',
      title: '安全中心 / Security Center',
      route: '/security-center',
      summary: '把漏洞、扫描任务、资产和工作台集中管理，用来做风险收敛和处置。',
      scenarios: ['查看漏洞列表和状态流转', '对扫描结果做人工复核和优先级判断'],
    },
    {
      icon: 'fas fa-robot',
      title: 'AI 助手 / AI Assistant',
      route: '/ai-assistant',
      summary: '适合做解释、归纳、草拟测试思路和消费知识库上下文，但不替代原始证据。',
      scenarios: ['让 AI 总结请求链路和可疑点', '基于知识库和历史上下文生成排查方案'],
    },
    {
      icon: 'fas fa-book-open',
      title: '知识库管理 / RAG',
      route: '/rag-management',
      summary: '存放 SOP、项目资料、接口约定和内部经验，让 AI 回答时有稳定上下文。',
      scenarios: ['沉淀测试手册、资产说明、项目背景', '减少同类问题的重复解释成本'],
    },
    {
      icon: 'fas fa-diagram-project',
      title: '工作流 / Workflow Studio',
      route: '/workflow-studio',
      summary: '把重复的侦察、校验、通知、整理动作串成自动化流程。',
      scenarios: ['将固定测试步骤做成可复用流程', '把工作流暴露为 AI 助手可调用工具'],
    },
    {
      icon: 'fas fa-toolbox',
      title: 'MCP 工具与插件',
      route: '/mcp-tools, /plugins',
      summary: 'MCP 负责接入外部能力，插件负责把自定义检测逻辑嵌入平台运行时。',
      scenarios: ['接第三方服务、扫描器或内部平台', '扩展流量分析、Agent 工具或自动化能力'],
    },
    {
      icon: 'fas fa-bullseye',
      title: '漏洞赏金 / Bug Bounty',
      route: '/bug-bounty',
      summary: '管理项目、监控范围变化、自动触发工作流，适合持续化运营目标面。',
      scenarios: ['维护项目资产和变更监控', '把赏金项目接入自动化发现链路'],
    },
  ],
  shortcutsTitle: '高频操作',
  shortcutsDescription: '这些操作会显著降低切换成本，适合先养成习惯。',
  shortcuts: [
    {
      keys: 'Ctrl/Cmd + K',
      description: '从任意主界面打开全局搜索，快速跳转页面、功能、消息和结果。',
    },
    {
      keys: '顶部帮助 -> 功能向导',
      description: '对当前页面做针对性引导，适合第一次进入复杂模块时使用。',
    },
    {
      keys: '顶部帮助 -> 帮助文档',
      description: '打开当前这份内置文档，快速确认模块职责和推荐工作流。',
    },
    { keys: '沉浸式挖洞模式', description: '收起不必要干扰，聚焦高频安全测试路径。' },
    {
      keys: '通知 / 消息中心',
      description: '跟踪工作流执行、自动化结果和 AI 反馈，避免异步结果丢失。',
    },
  ],
  faqTitle: '常见判断题',
  faqDescription: '遇到分叉时，先回答下面这些问题，通常就能知道该去哪里。',
  faqs: [
    {
      question: '为什么先看流量，再看 AI？',
      answer: '因为流量是原始证据，AI 只是推理器。没有证据，AI 很容易把猜测包装得像结论。',
    },
    {
      question: '什么时候该把流程做成工作流？',
      answer: '当步骤稳定、输入边界明确、重复次数足够高时，就该固化，否则继续人工探索更合适。',
    },
    {
      question: '知识库最适合放什么？',
      answer:
        '放高复用、低时效抖动的资料，例如 SOP、项目背景、接口规则、排查经验，而不是一次性聊天记录。',
    },
    {
      question: '插件和 MCP 工具怎么分？',
      answer:
        '插件更偏平台内部运行逻辑，MCP 更偏外部能力接入。前者解决“平台内怎么做”，后者解决“外部资源怎么接进来”。',
    },
  ],
  closingNote:
    '如果你已经知道目标是什么，但不知道入口，优先用全局搜索；如果你知道入口，却不知道怎么走，优先回到这份帮助文档。',
}

const enContent: HelpCenterContent = {
  badge: 'Built-in Help Center',
  title: 'Turn separate features into one executable security workflow',
  description:
    'This guide only documents capabilities that exist in the current build. The goal is not feature marketing, but faster decisions about which module should handle the next step.',
  releaseNote: 'This help content ships with the desktop app and remains available offline.',
  stats: [
    { label: 'Entry points', value: 'Top bar + sidebar' },
    { label: 'Suggested flow', value: 'Collect -> Analyze -> Reproduce -> Automate' },
    { label: 'Operating model', value: 'AI, workflow, plugin collaboration' },
  ],
  workflowTitle: 'Suggested path',
  workflowDescription:
    'From first principles, a security platform works only when reliable inputs are turned into reproducible conclusions.',
  workflowSteps: [
    {
      title: '1. Capture raw facts in Traffic Analysis',
      description:
        'Start with requests, responses, certificates, context, and reproducible samples. Weak inputs distort everything downstream.',
    },
    {
      title: '2. Triage and prioritize in Security Center',
      description:
        'Separate noise from real risk, then manage findings, scan tasks, assets, and investigation context in one place.',
    },
    {
      title: '3. Reproduce hypotheses with Intruder, replay, and compare',
      description:
        'The real quality threshold is converting “might be vulnerable” into “can be reproduced consistently.”',
    },
    {
      title: '4. Persist useful knowledge with AI, RAG, and workflows',
      description:
        'Automate repeatable steps with workflows, store reusable context in RAG, and move durable logic into tools or plugins.',
    },
  ],
  modulesTitle: 'Core modules',
  modulesDescription:
    'Each module should solve one class of problem. Once the responsibility is clear, the right entry point becomes obvious.',
  modules: [
    {
      icon: 'fas fa-chart-line',
      title: 'Dashboard',
      route: '/dashboard',
      summary: 'Use it for system-wide awareness: task volume, module health, and recent activity.',
      scenarios: [
        'Get a fast snapshot after opening the app',
        'Check whether scans, plugins, or knowledge data look unhealthy',
      ],
    },
    {
      icon: 'fas fa-network-wired',
      title: 'Traffic Analysis',
      route: '/traffic',
      summary:
        'The closest layer to ground truth for capture, filtering, replay, and HTTP attack validation.',
      scenarios: [
        'Proxy traffic capture, replay, and response comparison',
        'Validate auth bypass, parameter abuse, and input handling issues',
      ],
    },
    {
      icon: 'fas fa-shield-virus',
      title: 'Security Center',
      route: '/security-center',
      summary:
        'Centralize findings, scan tasks, assets, and workbench context for triage and disposition.',
      scenarios: [
        'Review finding states and priorities',
        'Perform human validation on scan output before action',
      ],
    },
    {
      icon: 'fas fa-robot',
      title: 'AI Assistant',
      route: '/ai-assistant',
      summary:
        'Best for explanation, summarization, and drafting investigation ideas, but not as a substitute for evidence.',
      scenarios: [
        'Summarize suspicious request chains and attack surface',
        'Generate investigation plans from stored context',
      ],
    },
    {
      icon: 'fas fa-book-open',
      title: 'RAG Management',
      route: '/rag-management',
      summary:
        'Store SOPs, project context, interface conventions, and internal knowledge so AI answers stay grounded.',
      scenarios: [
        'Persist manuals, environment notes, and internal references',
        'Reduce repeated explanation work on recurring issues',
      ],
    },
    {
      icon: 'fas fa-diagram-project',
      title: 'Workflow Studio',
      route: '/workflow-studio',
      summary:
        'Chain repeatable recon, validation, notification, and orchestration tasks into reusable flows.',
      scenarios: [
        'Convert stable manual procedures into automation',
        'Expose workflows as callable tools for the AI assistant',
      ],
    },
    {
      icon: 'fas fa-toolbox',
      title: 'MCP Tools and Plugins',
      route: '/mcp-tools, /plugins',
      summary:
        'Use MCP for external capability integration and plugins for platform-native runtime logic.',
      scenarios: [
        'Connect external services, scanners, or internal systems',
        'Extend traffic analysis, agent tooling, or automation behavior',
      ],
    },
    {
      icon: 'fas fa-bullseye',
      title: 'Bug Bounty',
      route: '/bug-bounty',
      summary:
        'Track programs, watch scope changes, and trigger workflows for continuous target operations.',
      scenarios: [
        'Manage target assets and change monitoring',
        'Attach bounty programs to automated discovery pipelines',
      ],
    },
  ],
  shortcutsTitle: 'High-frequency actions',
  shortcutsDescription: 'These are the habits that reduce switching cost the most.',
  shortcuts: [
    {
      keys: 'Ctrl/Cmd + K',
      description:
        'Open global search from the main app to jump to pages, functions, messages, and results quickly.',
    },
    {
      keys: 'Top bar Help -> Feature Guide',
      description:
        'Launch a page-specific walkthrough when you enter a complex module for the first time.',
    },
    {
      keys: 'Top bar Help -> Documentation',
      description:
        'Open this built-in help window to re-check responsibilities and the suggested workflow.',
    },
    {
      keys: 'Immersive Drill Mode',
      description:
        'Reduce interface noise and keep the focus on the high-frequency offensive workflow.',
    },
    {
      keys: 'Notifications / Message Center',
      description:
        'Track workflow output, automation events, and AI responses without losing async results.',
    },
  ],
  faqTitle: 'Common decision points',
  faqDescription:
    'When you are unsure where to go next, these questions usually resolve the branch quickly.',
  faqs: [
    {
      question: 'Why look at traffic before asking AI?',
      answer:
        'Because traffic is evidence and AI is a reasoning layer. Without evidence, AI can only give polished speculation.',
    },
    {
      question: 'When should a manual process become a workflow?',
      answer:
        'When the steps are stable, the input boundary is clear, and repetition is high enough to justify automation.',
    },
    {
      question: 'What belongs in the knowledge base?',
      answer:
        'Reusable and relatively stable material: SOPs, project context, interface rules, and investigation patterns, not disposable chat logs.',
    },
    {
      question: 'How do plugins differ from MCP tools?',
      answer:
        'Plugins shape logic inside the platform runtime. MCP tools connect outside capabilities into the platform.',
    },
  ],
  closingNote:
    'If you know the goal but not the entry point, start with global search. If you know the entry point but not the workflow, come back to this document.',
}

export function getHelpCenterContent(locale: string): HelpCenterContent {
  return locale.startsWith('zh') ? zhContent : enContent
}
