import {
  enAiAssistantGovernanceEntry,
  enAiAssistantSessionsEntry,
  enBuiltinSkillsEntry,
  enMcpServersEntry,
  zhAiAssistantGovernanceEntry,
  zhAiAssistantSessionsEntry,
  zhBuiltinSkillsEntry,
  zhMcpServersEntry,
} from './helpCenterAssistantToolsContent'
import {
  enBugBountyFindingsEntry,
  enBugBountyKnowledgeEntry,
  enBugBountyOperationsEntry,
  zhBugBountyFindingsEntry,
  zhBugBountyKnowledgeEntry,
  zhBugBountyOperationsEntry,
} from './helpCenterBugBountyContent'
import {
  enPluginLifecycleEntry,
  enPluginStoreReviewEntry,
  enRagCollectionsEntry,
  enRagIngestQueryEntry,
  zhPluginLifecycleEntry,
  zhPluginStoreReviewEntry,
  zhRagCollectionsEntry,
  zhRagIngestQueryEntry,
} from './helpCenterExtensionKnowledgeContent'
import {
  enSecurityFindingsEntry,
  enSecurityWorkbenchEntry,
  enWorkflowCanvasEntry,
  enWorkflowExecutionEntry,
  zhSecurityFindingsEntry,
  zhSecurityWorkbenchEntry,
  zhWorkflowCanvasEntry,
  zhWorkflowExecutionEntry,
} from './helpCenterSecurityWorkflowContent'
import { enTrafficOastEntry, zhTrafficOastEntry } from './helpCenterOastContent'

export interface HelpCenterStat {
  label: string
  value: string
}

export interface HelpCenterWorkflowStep {
  id: string
  title: string
  description: string
}

export interface HelpCenterFeatureEntry {
  id: string
  icon: string
  title: string
  route: string
  summary: string
  capabilities: string[]
  operations: string[]
  detailSections?: HelpCenterFeatureDetailSection[]
}

export interface HelpCenterFeatureDetailCodeBlock {
  id: string
  label: string
  language: string
  code: string
}

export interface HelpCenterFeatureDetailSection {
  id: string
  title: string
  description?: string
  items?: string[]
  ordered?: boolean
  codeBlocks?: HelpCenterFeatureDetailCodeBlock[]
}

export interface HelpCenterCatalogSection {
  id: string
  badge: string
  title: string
  description: string
  entries: HelpCenterFeatureEntry[]
}

export interface HelpCenterFaq {
  id: string
  question: string
  answer: string
}

export interface HelpCenterContent {
  badge: string
  overviewTitle: string
  title: string
  description: string
  releaseNote: string
  stats: HelpCenterStat[]
  workflowTitle: string
  workflowDescription: string
  workflowSteps: HelpCenterWorkflowStep[]
  catalogSections: HelpCenterCatalogSection[]
  faqTitle: string
  faqDescription: string
  faqs: HelpCenterFaq[]
  closingNote: string
}

const zhContent: HelpCenterContent = {
  badge: 'Sentinel AI 内置帮助中心',
  overviewTitle: '文档概览',
  title: '按真实入口理解 Sentinel AI，而不是按概念猜测',
  description:
    '这份帮助文档覆盖当前桌面端已经存在的主要功能入口。文档按“全局入口、核心功能、工具与内容、系统与协作”组织，每个条目都同时给出功能说明和最短操作路径。',
  releaseNote: '文档内容随当前桌面端版本一起发布，可离线查看。',
  stats: [
    { label: '导航结构', value: '左侧目录 + 右侧正文' },
    { label: '阅读方式', value: '先定入口，再看步骤' },
    { label: '文档目标', value: '说明真实功能，不做概念铺陈' },
  ],
  workflowTitle: '推荐使用路径',
  workflowDescription:
    '从第一性原理出发，平台的主线只有一条：先拿到可靠输入，再把输入变成可复现结论，最后把结论沉淀成可重复执行的能力。',
  workflowSteps: [
    {
      id: 'workflow-capture',
      title: '1. 先采集事实',
      description:
        '优先去流量分析、资产或项目模块拿到请求、响应、样本、上下文和范围定义。没有事实输入，后面的判断不稳定。',
    },
    {
      id: 'workflow-triage',
      title: '2. 再做筛选与定级',
      description:
        '把采集到的事实放进安全中心、漏洞赏金或消息中心进行筛选，区分噪音、线索和真正值得跟进的问题。',
    },
    {
      id: 'workflow-reproduce',
      title: '3. 把假设变成复现结果',
      description:
        '用重放器、爆破器、比较器、工作流或插件，把“看起来有问题”收敛成“能稳定复现的问题”。',
    },
    {
      id: 'workflow-persist',
      title: '4. 最后沉淀经验',
      description:
        '把稳定流程放进工作流，把资料放进知识库，把扩展逻辑做成插件或工具，把通知规则接到协作链路中。',
    },
  ],
  catalogSections: [
    {
      id: 'global-controls',
      badge: '全局入口',
      title: '全局入口与快捷操作',
      description:
        '这些能力不绑定单一业务页面，作用是降低切换成本、缩短定位路径，并把常用动作收敛成固定入口。',
      entries: [
        {
          id: 'feature-global-search',
          icon: 'fas fa-magnifying-glass',
          title: '全局搜索',
          route: '/search，或 Ctrl/Cmd + K',
          summary:
            '全局搜索是整个桌面端的统一入口。它既能搜页面和常用功能，也支持命令式输入、固定快捷入口和最近命令。',
          capabilities: [
            '搜索页面、功能、消息、结果和高频入口。',
            '支持命令式输入，例如主题切换、通知规则、漏洞筛选等命令。',
            '支持固定快捷入口、最近搜索、最近命令和推荐入口。',
          ],
          operations: [
            '在任意主界面按 Ctrl/Cmd + K，或直接进入 /search。',
            '输入关键词直接搜索；如果想走命令式路径，输入 theme dark、notify rules、finding critical 之类命令。',
            '命中结果后直接跳转；常用结果可以固定为快捷入口，减少重复搜索。',
          ],
        },
        {
          id: 'feature-help-guide',
          icon: 'fas fa-circle-question',
          title: '帮助与功能向导',
          route: '顶部帮助菜单',
          summary: '顶部帮助菜单负责两件事：对当前页面做功能向导，以及打开这份内置帮助文档。',
          capabilities: [
            '为复杂页面提供针对性的上手引导。',
            '打开独立帮助文档窗口，适合做模块职责和操作路径确认。',
            '帮助入口不依赖网络，可以离线查看。',
          ],
          operations: [
            '点击顶部问号按钮。',
            '如果你需要了解当前页面怎么操作，选择“功能向导”；如果你需要确认模块职责和完整说明，选择“帮助文档”。',
            '在帮助文档中用左侧目录定位功能，再在右侧正文查看功能说明和操作说明。',
          ],
        },
        {
          id: 'feature-immersive-drill',
          icon: 'fas fa-crosshairs',
          title: '沉浸式挖洞模式',
          route: '顶部十字准星按钮',
          summary:
            '沉浸式挖洞模式会把界面切到更聚焦的安全测试路径，减少不必要干扰，让流量分析和安全中心更适合连续操作。',
          capabilities: [
            '切换到更聚焦的挖掘视图。',
            '在安全中心和流量分析中启用更轻量的聚焦工作方式。',
            '适合集中处理复现、筛选和验证任务。',
          ],
          operations: [
            '点击顶部十字准星按钮进入模式。',
            '进入后优先在流量分析和安全中心处理高频动作，减少页面切换。',
            '完成集中处理后再次点击按钮退出，回到完整工作区。',
          ],
        },
        {
          id: 'feature-notification-inbox',
          icon: 'fas fa-inbox',
          title: '消息收件箱与通知视图',
          route: '/notification-center，以及顶部收件箱/铃铛下拉',
          summary:
            '这个入口负责承接异步结果，包括消息、系统通知、流程反馈和后续跳转动作，避免后台执行结果丢失。',
          capabilities: [
            '顶部下拉可以快速查看消息和通知。',
            '消息中心支持按来源过滤、只看未读、标记已读和直接跳转目标。',
            '可配置桌面通知与声音偏好，并跳转到通知规则管理。',
          ],
          operations: [
            '先看顶部收件箱和铃铛，快速处理最新结果。',
            '如果需要批量筛选或回看历史，进入 /notification-center。',
            '在消息中心里按来源和未读状态过滤；需要对外发送通知时，再进入通知规则页配置通道。',
          ],
        },
        {
          id: 'feature-language-theme',
          icon: 'fas fa-palette',
          title: '语言与主题切换',
          route: '顶部语言/主题菜单，持久设置在 /settings',
          summary: '顶部按钮解决即时切换，系统设置负责持久保存主题、语言、字号和界面缩放。',
          capabilities: [
            '支持在当前会话中快速切换语言和主题。',
            '持久化外观配置由系统设置接管。',
            '适合在不同演示、排查和个人偏好之间快速切换。',
          ],
          operations: [
            '用顶部语言按钮切换中文/英文，用调色板按钮切换主题。',
            '如果你要长期保存字体大小、界面缩放或语言偏好，进入 /settings 的系统分类。',
            '修改后回到帮助文档或主界面验证整体阅读和操作体验。',
          ],
        },
      ],
    },
    {
      id: 'core-features',
      badge: '核心功能',
      title: '核心业务功能',
      description:
        '这一组是侧边栏主功能区，也是最接近安全工作主线的页面。这里关注的是事实采集、筛选、复现、协同和持续运营。',
      entries: [
        {
          id: 'feature-dashboard',
          icon: 'fas fa-chart-line',
          title: '总览 / Dashboard',
          route: '/dashboard',
          summary:
            '总览页的职责是给出系统总体态势，包括资产、漏洞、流量、AI 使用、工具统计、插件统计和数据库等概览信息。',
          capabilities: [
            '提供多张统计卡片和图表卡片。',
            '支持显示/隐藏卡片、拖拽重排和刷新数据。',
            '部分卡片可以直接跳转到扫描任务、漏洞等明细页面。',
          ],
          operations: [
            '进入 /dashboard 后先看整体统计，判断当前系统的热点在哪里。',
            '用右上角设置菜单显示或隐藏卡片，并按自己的工作顺序拖拽重排。',
            '当某一类指标异常时，点击对应入口继续进入安全中心、插件或工具页处理。',
          ],
        },
        {
          id: 'feature-security-center',
          icon: 'fas fa-shield-virus',
          title: '安全中心',
          route: '/security-center、/security-center/workbench、/vulnerabilities、/scan-tasks',
          summary:
            '安全中心负责把风险收敛到统一工作台中。当前主标签包括工作台、漏洞和 LLM 安全，用于做筛选、复核、跟进和处置。',
          capabilities: [
            '它是安全结果和调查过程的总入口，而不是单一列表页。',
            '工作台负责聚合案件和调查上下文。',
            '漏洞与 LLM 安全负责标准化结果处理，详细说明已拆到同目录下的独立文档项。',
          ],
          operations: [
            '如果你在推进案件调查，进入“安全中心 / 工作台”。',
            '如果你在处理结果、状态或 LLM 安全视图，进入“安全中心 / 漏洞与 LLM 安全”。',
            '当标准化结果仍然需要复现时，再回到流量分析。',
          ],
        },
        zhSecurityWorkbenchEntry,
        zhSecurityFindingsEntry,
        {
          id: 'feature-traffic-analysis',
          icon: 'fas fa-network-wired',
          title: '流量分析',
          route: '/traffic，以及爆破结果窗口 /intruder-results/:workspaceId',
          summary:
            '流量分析是离事实最近的一层。这里承载抓包、代理、拦截、重放、爆破、比较、OAST 回连和请求上下文处理，是验证问题的核心工作区。',
          capabilities: [
            '支持 Traffic Workbench 和传统标签页两种流量工作方式。',
            '包含代理、拦截、重放器、爆破器、比较器、历史记录、OAST 面板和辅助上下文能力。',
            '支持把请求发送到重放器、爆破器、比较器，并在独立窗口查看爆破结果。',
            '支持在“流量分析 -> 配置 -> 分析”中配置 OAST / Collaborator 服务，包括基地址、Worker API Key、轮询间隔、超时、自动保存和连通性测试。',
            '支持在 Repeater、Intruder 中生成并插入 OAST payload，并在 OAST 面板里同步命中、搜索记录与事件、删除事件、导出证据和回跳来源请求。',
            '完整的 Cloudflare Worker OAST 配置、部署、验证与排障文档已拆到同目录下的“流量分析 / OAST”。',
          ],
          operations: [
            '先配置代理或流量来源，进入流量历史拿到请求与响应事实。',
            '选中请求后按目标发送到重放器、爆破器或比较器，分别做修改复测、批量测试和差异比对。',
            '如果要验证 SSRF、XXE、存储型 XSS 或其他带外回连，在“配置 -> 分析”里先启用 OAST，填入 Worker 管理地址与 Worker API Key，再执行连接测试。',
            '如果你需要自建或排查 Cloudflare Worker OAST，直接打开同目录下的“流量分析 / OAST”文档项，不要在这里查零散步骤。',
            '先在 Worker 管理端生成 token 和 payload，再在 Repeater 或 Intruder 中点击“插入 OAST Payload”或直接粘贴 URL，发送后回到 OAST 面板同步命中事件。',
            '在 OAST 面板中按 token、域名、来源请求或事件内容筛选命中，必要时导出当前结果为 JSON 证据。',
            '在 OAST 面板中查看 token、payload、命中次数、事件详情和来源请求；事件里至少要核对 Host、URL、IP、UA、时间与来源请求，必要时删除噪声事件或直接打开来源请求继续分析。',
            '确认可复现后，把有效结论带回安全中心、漏洞赏金或工作流继续处理。',
          ],
        },
        zhTrafficOastEntry,
        {
          id: 'feature-ai-assistant',
          icon: 'fas fa-brain',
          title: 'AI 助手',
          route: '/ai-assistant',
          summary:
            'AI 助手是统一的对话式工作区，适合做解释、归纳、生成思路、消费知识库上下文和联动工具，但它不替代原始证据。',
          capabilities: [
            '它是多会话工作区和 AI 治理入口，而不只是一个聊天面板。',
            '可结合知识库、历史上下文和工具能力进行分析。',
            '会话/角色与强制规则/Turn 日志的详细说明已拆到同目录下的独立文档项。',
          ],
          operations: [
            '如果你在组织会话、切角色或切换上下文，进入“AI 助手 / 会话与角色”。',
            '如果你在排查行为边界或回看单轮执行，进入“AI 助手 / 强制规则与 Turn 日志”。',
            'AI 给出建议后，仍然要回到业务页验证，不要把判断停留在对话层。',
          ],
        },
        zhAiAssistantSessionsEntry,
        zhAiAssistantGovernanceEntry,
        {
          id: 'feature-workflow-studio',
          icon: 'fas fa-diagram-project',
          title: '工作流 / Workflow Studio',
          route: '/workflow-studio',
          summary:
            '工作流页面负责把重复动作变成流程。它覆盖工作流列表、模板、节点画布、参数抽屉、日志、执行历史、导入导出和调度运行。',
          capabilities: [
            '它是流程设计和流程运行的总入口，不该只被理解成画布编辑器。',
            '支持创建、加载、克隆、删除工作流和模板。',
            '画布/参数与执行/历史的详细说明已经拆到同目录下的独立文档项。',
          ],
          operations: [
            '如果你在定义流程结构，进入“工作流 / 画布与参数”。',
            '如果你在排查运行、日志或历史，进入“工作流 / 执行与历史”。',
            '流程稳定后再考虑把它沉淀成模板或工具。',
          ],
        },
        zhWorkflowCanvasEntry,
        zhWorkflowExecutionEntry,
        {
          id: 'feature-bug-bounty',
          icon: 'fas fa-bullseye',
          title: '漏洞赏金 / Bug Bounty',
          route: '/bug-bounty',
          summary:
            '漏洞赏金模块负责持续化运营目标面。它不只是放项目列表，而是把项目、资产、API、发现、知识、提交通道、变更监控和工作流都串起来。',
          capabilities: [
            '标签页覆盖 Programs、Assets、API Inventory、Findings、Knowledge、Submissions、Statistics、Import/Export、Templates、Changes、Workflows、Monitor。',
            '支持按项目管理资产与发现，并维护提交与收益数据。',
            '支持将变更事件、监控和工作流模板接入持续化流程。',
          ],
          operations: [
            '先选择或创建项目，再维护范围内资产和 API 目录。',
            '把发现记录、知识条目和提交动作绑定到项目维度，形成持续追踪链路。',
            '如果任务已经聚焦到某一环节，直接进入同目录下的 Findings、Knowledge 或 持续监控 文档项查看该部分的详细说明。',
          ],
        },
        zhBugBountyFindingsEntry,
        zhBugBountyKnowledgeEntry,
        zhBugBountyOperationsEntry,
      ],
    },
    {
      id: 'tools-and-content',
      badge: '工具与内容',
      title: '工具、扩展与知识内容',
      description:
        '这一组负责把能力接进平台或沉淀进平台，包括 MCP、内置工具、插件、字典、知识库和编码处理能力。',
      entries: [
        {
          id: 'feature-mcp-tools',
          icon: 'fas fa-tools',
          title: '应用工具 / MCP Tools',
          route: '/mcp-tools',
          summary:
            '工具页是能力接入层。当前页面提供内置工具、MCP Servers、技能管理等视图，也支持查看服务器详情、工具参数和导入配置。',
          capabilities: [
            '它是平台能力接入的总入口，不是单一的 server 配置页。',
            '内置工具视图可查看 builtin、workflow、plugin 三类来源。',
            'MCP Servers 与 Builtin/Skills 的详细说明已拆到同目录下的独立文档项。',
          ],
          operations: [
            '如果你在接外部能力，进入“应用工具 / MCP Servers”。',
            '如果你在看平台已有工具或技能层能力，进入“应用工具 / 内置工具与 Skills”。',
            '能力边界确认后，再回 AI 助手或工作流实际消费这些工具。',
          ],
        },
        zhMcpServersEntry,
        zhBuiltinSkillsEntry,
        {
          id: 'feature-cyberchef',
          icon: 'fas fa-terminal',
          title: '编码解码 / CyberChef',
          route: '/cyberchef',
          summary:
            '编码解码页用于做文本、字节和常见格式转换，适合处理样本、载荷、密文片段和调试中的中间结果。',
          capabilities: [
            '适合做编码、解码、格式整理和快速内容变换。',
            '适合在流量分析、插件调试和漏洞验证过程中处理中间数据。',
            '是独立工具页，适合临时计算和结果校验。',
          ],
          operations: [
            '把待处理内容粘贴进 CyberChef。',
            '选择或组合需要的转换步骤，观察输出结果是否符合预期。',
            '把转换后的结果带回流量分析、插件、工作流或提交材料中使用。',
          ],
        },
        {
          id: 'feature-dictionary',
          icon: 'fas fa-book',
          title: '字典管理',
          route: '/dictionary',
          summary:
            '字典管理用于维护词表、规则和结构化模板，服务于爆破、生成、匹配和自动化处理场景。',
          capabilities: [
            '支持维护字典项、规则模板和结构化条目。',
            '支持批量编辑和规则化生成。',
            '适合给爆破器、工作流、插件或搜索能力提供稳定输入。',
          ],
          operations: [
            '先确定字典是用于词表、规则还是模板生成。',
            '创建或编辑条目，必要时使用批量能力统一调整。',
            '在后续的爆破、工作流或插件逻辑里复用这些字典，而不是重复手写输入。',
          ],
        },
        {
          id: 'feature-plugin-management',
          icon: 'fas fa-puzzle-piece',
          title: '插件管理',
          route: '/plugins',
          summary:
            '插件管理负责自定义扩展能力，覆盖新增、上传、AI 生成、分类浏览、插件商店、审核、测试、启停和代码查看。',
          capabilities: [
            '它是插件总入口，负责把生命周期、商店和审核能力组织在同一页面。',
            '支持分类筛选、收藏、批量启停、删除、审核和商店安装。',
            '详细的生命周期说明，以及商店/审核治理说明，已经拆到同目录下的独立文档项。',
          ],
          operations: [
            '先决定你现在处理的是插件生命周期，还是商店/审核问题。',
            '如果是创建、上传、AI 生成、测试、启停，进入“插件管理 / 生命周期”。',
            '如果是来源治理和准入治理，进入“插件管理 / 商店与审核”。',
          ],
        },
        zhPluginLifecycleEntry,
        zhPluginStoreReviewEntry,
        {
          id: 'feature-rag-management',
          icon: 'fas fa-database',
          title: '知识库管理 / RAG',
          route: '/rag-management',
          summary:
            '知识库管理负责组织 AI 可消费的长期资料。页面包含集合统计、搜索过滤、集合创建、文档导入、激活切换和查询能力。',
          capabilities: [
            '它是知识结构和资料消费的总入口，而不是只看文档上传结果。',
            '支持集合管理、激活切换、文档导入、集合查询和详情查看。',
            '集合边界和导入/查询链路已经拆到同目录下的独立文档项。',
          ],
          operations: [
            '如果你在处理知识结构问题，进入“知识库管理 / 集合管理”。',
            '如果你在处理文档导入和召回验证问题，进入“知识库管理 / 导入与查询”。',
            '完成集合或查询侧验证后，再回到 AI 助手确认端到端效果。',
          ],
        },
        zhRagCollectionsEntry,
        zhRagIngestQueryEntry,
      ],
    },
    {
      id: 'system-and-collaboration',
      badge: '系统与协作',
      title: '系统配置、通知与协作',
      description:
        '这一组处理的是平台治理和运行配置，包括智能体、系统设置、通知规则、消息中心和性能观察等能力。',
      entries: [
        {
          id: 'feature-agent-management',
          icon: 'fas fa-robot',
          title: '智能体管理',
          route: '/agent-management',
          summary:
            '智能体管理页把交互型 Agent、后台型 Agent 以及共享运行环境放在同一个入口，当前主要分为 Agent 管理和运行环境两个标签。',
          capabilities: [
            'Agent 管理标签负责统一维护 Agent 注册信息。',
            '运行环境标签负责终端执行环境、工作目录、附件、Subagent 超时等参数。',
            '适合统一治理平台内各类智能体的运行方式。',
          ],
          operations: [
            '进入 /agent-management 后先确认你要改的是 Agent 本身，还是运行环境。',
            '在 Agent 管理标签处理定义和注册，在运行环境标签处理目录、权限和超时等执行参数。',
            '修改后回到 AI 助手或相关自动化流程验证行为是否符合预期。',
          ],
        },
        {
          id: 'feature-settings',
          icon: 'fas fa-cog',
          title: '系统设置',
          route: '/settings',
          summary:
            '系统设置是平台的集中配置入口，当前按 AI、RAG、数据库、系统、网络、安全六大分类组织。',
          capabilities: [
            'AI 分类负责模型、Provider、连接测试、默认模型与统计清理。',
            'RAG 分类负责 Embedding、分块和检索参数配置。',
            '数据库、系统、网络、安全分类负责存储、界面、代理和安全策略配置。',
          ],
          operations: [
            '先选中你要调整的分类，而不是在整页里盲目查找。',
            '修改参数后优先执行页面内的连接测试、能力刷新或校验动作。',
            '确认有效后保存配置，并回到对应业务页验证设置是否真正影响运行结果。',
          ],
        },
        {
          id: 'feature-notification-rules',
          icon: 'fas fa-sliders',
          title: '通知规则管理',
          route: '/notifications',
          summary:
            '通知规则页负责定义“什么事件，以什么通道，发到哪里”。当前支持飞书、钉钉、企业微信、Webhook 和邮件等通道。',
          capabilities: [
            '支持新增、编辑、启停和删除通知规则。',
            '支持多种发送通道及其专属配置项。',
            '支持测试连接，验证配置是否能真正送达。',
          ],
          operations: [
            '进入 /notifications 后新增规则，先填写通知类型和说明。',
            '选择发送通道并填入对应的 webhook、密钥或邮件配置，然后执行连接测试。',
            '测试通过后启用规则，再回到消息中心或业务页观察事件是否按预期触发。',
          ],
        },
        {
          id: 'feature-notification-center',
          icon: 'fas fa-bell',
          title: '消息中心',
          route: '/notification-center',
          summary:
            '消息中心是异步结果的统一浏览页，负责把消息和通知按类别、来源和未读状态集中管理。',
          capabilities: [
            '支持 All、Messages、Notifications 分类统计。',
            '支持来源过滤、未读筛选、批量清理和标记已读。',
            '支持桌面通知、声音等偏好设置，并可跳转通知规则页。',
          ],
          operations: [
            '先按分类和来源过滤，把真正需要处理的消息筛出来。',
            '打开具体条目查看详情和目标跳转，处理后标记已读或清理。',
            '如果消息分发本身需要调整，直接跳转到通知规则页继续配置。',
          ],
        },
        {
          id: 'feature-performance-monitor',
          icon: 'fas fa-gauge-high',
          title: '性能监控',
          route: '/performance',
          summary:
            '性能监控页用于观察运行态负载和性能表现，帮助判断系统当前是否存在资源瓶颈或异常抖动。',
          capabilities: [
            '提供独立的运行性能观察入口。',
            '适合在高频抓包、AI 调用、工作流执行后观察系统状态。',
            '可作为定位性能问题的辅助视图。',
          ],
          operations: [
            '当你怀疑系统卡顿、任务拥塞或资源异常时进入 /performance。',
            '对照当前正在执行的流量、AI、工作流或插件任务观察性能表现。',
            '确认瓶颈位置后回到对应模块调整配置、并发或执行方式。',
          ],
        },
      ],
    },
  ],
  faqTitle: '常见判断题',
  faqDescription: '当你不确定该从哪里开始时，先回答这些问题，通常就能定位正确入口。',
  faqs: [
    {
      id: 'faq-traffic-before-ai',
      question: '为什么大多数问题都建议先看流量，而不是先问 AI？',
      answer:
        '因为流量、样本和上下文是证据，AI 只是推理层。没有证据时，AI 的输出只能作为假设，不能替代验证。',
    },
    {
      id: 'faq-search-or-help',
      question: '什么时候用全局搜索，什么时候看帮助文档？',
      answer:
        '如果你知道目标，但不知道入口，用全局搜索；如果你已经到了入口，但不知道这个页面负责什么、该怎么操作，就看帮助文档。',
    },
    {
      id: 'faq-workflow-or-plugin',
      question: '工作流、插件、MCP 工具该怎么区分？',
      answer:
        '工作流解决“如何把多个动作串起来”，插件解决“平台内部的扩展逻辑是什么”，MCP 工具解决“外部能力怎么接进来”。三者职责不同，不要混用。',
    },
    {
      id: 'faq-settings-boundary',
      question: '为什么很多能力在顶部和系统设置里都能改？',
      answer:
        '顶部入口负责即时切换，系统设置负责持久化和细粒度配置。一个解决“现在就要切”，一个解决“以后一直这样跑”。',
    },
    {
      id: 'faq-oast-not-dnslog',
      question: '为什么 OAST 有时没有命中，或者命中比预期少？',
      answer:
        '因为 Worker 版 OAST 本质上是 HTTP/HTTPS 回连平台，不是权威 DNSLog。目标如果只做了 DNS 解析、没有继续发起 HTTP/HTTPS 请求，Worker 不会看到事件。另一个高频原因是 DNS 没有同时配置 oast 与 *.oast、没有开启代理，或者 Worker Route 没同时覆盖根域和通配符子域。',
    },
    {
      id: 'faq-feature-entitlement-visibility',
      question: '为什么帮助文档里有些功能写了，但我的侧边栏里看不到入口？',
      answer:
        '因为帮助文档描述的是产品已有能力，而实际导航入口还受许可证、功能授权或当前运行配置控制。最典型的例子是漏洞赏金：文档会说明它负责什么，但侧边栏只有在授权开启后才显示。遇到这种情况，先区分“功能不存在”和“当前实例未开放入口”。',
    },
  ],
  closingNote:
    '这份文档的正确使用方式不是从头读到尾，而是先在左侧找到你当前的问题属于哪一类，再沿着对应条目的操作说明往下走。',
}

const enContent: HelpCenterContent = {
  badge: 'Built-in Help Center',
  overviewTitle: 'Overview',
  title: 'Understand Sentinel AI by real entry points, not abstract concepts',
  description:
    'This guide covers the major capabilities that exist in the current desktop build. It is organized by global entry points, core workflows, tools and content, and system operations. Every entry explains both what it does and how to use it.',
  releaseNote:
    'This help content ships with the current desktop build and remains available offline.',
  stats: [
    { label: 'Navigation', value: 'Left tree + right document' },
    { label: 'Reading mode', value: 'Pick the entry, then follow the steps' },
    { label: 'Goal', value: 'Explain real features, not product slogans' },
  ],
  workflowTitle: 'Suggested path',
  workflowDescription:
    'From first principles, the platform has one practical path: collect reliable input, turn it into reproducible conclusions, then persist what is repeatable.',
  workflowSteps: [
    {
      id: 'workflow-capture',
      title: '1. Capture facts first',
      description:
        'Start with traffic, scope, samples, context, and actual artifacts. Without reliable input, later analysis becomes unstable.',
    },
    {
      id: 'workflow-triage',
      title: '2. Triage and prioritize',
      description:
        'Move captured facts into the Security Center, Bug Bounty, or notifications flow to separate noise from work that deserves follow-up.',
    },
    {
      id: 'workflow-reproduce',
      title: '3. Turn hypotheses into reproducible results',
      description:
        'Use repeater, intruder, comparer, workflows, and plugins to convert “looks suspicious” into “can be reproduced reliably.”',
    },
    {
      id: 'workflow-persist',
      title: '4. Persist what works',
      description:
        'Put repeatable procedures into workflows, reusable knowledge into RAG, and durable logic into plugins, tools, and notification chains.',
    },
  ],
  catalogSections: [
    {
      id: 'global-controls',
      badge: 'Global',
      title: 'Global entry points and shortcuts',
      description:
        'These features are not tied to one business page. Their job is to reduce switching cost and expose common actions through stable entry points.',
      entries: [
        {
          id: 'feature-global-search',
          icon: 'fas fa-magnifying-glass',
          title: 'Global Search',
          route: '/search, or Ctrl/Cmd + K',
          summary:
            'Global Search is the unified entry point across the desktop app. It supports page search, feature lookup, command-style input, pinned shortcuts, and recent commands.',
          capabilities: [
            'Search pages, features, messages, and common entry points.',
            'Accept command-style input such as theme switching or notification rule navigation.',
            'Expose pinned shortcuts, recent searches, recent commands, and featured entries.',
          ],
          operations: [
            'Press Ctrl/Cmd + K from any main page, or open /search directly.',
            'Type keywords for lookup or command-style input for direct actions.',
            'Open the matching result immediately and pin high-frequency targets when they become repetitive.',
          ],
        },
        {
          id: 'feature-help-guide',
          icon: 'fas fa-circle-question',
          title: 'Help and Feature Guide',
          route: 'Top-bar Help menu',
          summary:
            'The Help menu serves two purposes: page-specific walkthroughs and access to this standalone help document.',
          capabilities: [
            'Launch a guided introduction for the current page.',
            'Open the built-in help document in a separate help window.',
            'Keep documentation available offline with the app build.',
          ],
          operations: [
            'Click the top-bar Help button.',
            'Choose the feature guide when you need page-specific onboarding, or choose documentation for module responsibilities and full usage notes.',
            'Use the left navigation tree in the help window to jump to the relevant feature entry.',
          ],
        },
        {
          id: 'feature-immersive-drill',
          icon: 'fas fa-crosshairs',
          title: 'Immersive Drill Mode',
          route: 'Top-bar crosshair button',
          summary:
            'Immersive Drill Mode shifts the interface into a more focused security workflow with fewer distractions around high-frequency testing tasks.',
          capabilities: [
            'Switch the interface into a focused drilling state.',
            'Provide lighter, more concentrated traffic and security workflows.',
            'Fit long sessions of verification and triage work.',
          ],
          operations: [
            'Click the crosshair button in the top bar to enter the mode.',
            'Use Traffic Analysis and Security Center inside the focused workflow while noise is reduced.',
            'Click the button again when you want to return to the full workspace.',
          ],
        },
        {
          id: 'feature-notification-inbox',
          icon: 'fas fa-inbox',
          title: 'Inbox and Notification Views',
          route: '/notification-center and top-bar inbox/bell dropdowns',
          summary:
            'This entry point handles asynchronous outcomes such as messages, system notifications, workflow output, and target jumps so background results are not lost.',
          capabilities: [
            'Show recent messages and notifications from top-bar dropdowns.',
            'Filter history by source and unread state in the Notification Center.',
            'Configure desktop and sound preferences, then jump to rule management.',
          ],
          operations: [
            'Use the top-bar inbox and bell for quick triage of recent results.',
            'Open /notification-center when you need filtering, history review, or bulk handling.',
            'Move to notification rule management only when delivery behavior itself needs to change.',
          ],
        },
        {
          id: 'feature-language-theme',
          icon: 'fas fa-palette',
          title: 'Language and Theme Switching',
          route: 'Top-bar language/theme menus, persisted settings in /settings',
          summary:
            'The top bar handles immediate switching, while the Settings page owns persisted appearance, language, font size, and scale configuration.',
          capabilities: [
            'Switch language and theme quickly in the current session.',
            'Persist appearance and language behavior in system settings.',
            'Support fast context changes across demo, analysis, and personal preference workflows.',
          ],
          operations: [
            'Use the top-bar language menu to switch locale and the palette menu to switch theme.',
            'Open /settings when you want the change to persist together with font size or UI scale.',
            'Return to a working page and verify readability and layout after the change.',
          ],
        },
      ],
    },
    {
      id: 'core-features',
      badge: 'Core',
      title: 'Core operational features',
      description:
        'These are the main items in the left primary navigation. They are closest to the security workflow itself: capture, triage, reproduce, coordinate, and operate continuously.',
      entries: [
        {
          id: 'feature-dashboard',
          icon: 'fas fa-chart-line',
          title: 'Dashboard',
          route: '/dashboard',
          summary:
            'The Dashboard provides a system-wide snapshot: assets, vulnerabilities, traffic, AI usage, tools, plugins, and storage-level signals.',
          capabilities: [
            'Expose multiple statistic and chart cards.',
            'Support show/hide, drag-reorder, and manual refresh.',
            'Allow drill-down from some cards into detailed pages.',
          ],
          operations: [
            'Open /dashboard and identify which area currently needs attention.',
            'Adjust visible cards and ordering so the page matches your working priorities.',
            'Jump into detailed pages when a specific metric or category looks abnormal.',
          ],
        },
        {
          id: 'feature-security-center',
          icon: 'fas fa-shield-virus',
          title: 'Security Center',
          route: '/security-center, /security-center/workbench, /vulnerabilities, /scan-tasks',
          summary:
            'Security Center consolidates risk handling. The main tabs currently focus on the workbench, vulnerabilities, and LLM security so triage and follow-up stay in one place.',
          capabilities: [
            'It is the main security investigation and result-handling entry rather than a single list view.',
            'Workbench organizes cases and investigation context.',
            'Findings and LLM security handle standardized result streams, and their detailed operating model now lives in separate help entries in the same directory.',
          ],
          operations: [
            'Open "Security Center / Workbench" when the task is still case-driven.',
            'Open "Security Center / Findings and LLM Security" when the task is already result-driven.',
            'Return to Traffic Analysis when standardized results still need reproduction.',
          ],
        },
        enSecurityWorkbenchEntry,
        enSecurityFindingsEntry,
        {
          id: 'feature-traffic-analysis',
          icon: 'fas fa-network-wired',
          title: 'Traffic Analysis',
          route: '/traffic, plus the standalone intruder result window',
          summary:
            'Traffic Analysis is the closest layer to evidence. It covers capture, proxying, interception, replay, intruder, comparison, OAST callbacks, and traffic context handling.',
          capabilities: [
            'Support both a Traffic Workbench mode and legacy tab layout.',
            'Include proxy, intercept, repeater, intruder, comparer, history, OAST, and context support.',
            'Allow requests to be transferred into repeater, intruder, comparer, and standalone result windows.',
            'Let you configure the OAST / Collaborator service under "Traffic Analysis -> Settings -> Analysis", including base URL, Worker API key, polling interval, timeout, automatic saving, and connectivity testing.',
            'Let Repeater and Intruder generate and insert OAST payloads, then inspect hits, search records and events, delete callback noise, export evidence, and jump back to source requests from the OAST panel.',
            'Use the separate "Traffic Analysis / OAST" document in the same directory for the full Cloudflare Worker OAST deployment, validation, and troubleshooting guide.',
          ],
          operations: [
            'Configure the traffic source and capture requests and responses first.',
            'Send interesting requests into repeater, intruder, or comparer depending on whether you need mutation, batching, or diffing.',
            'When validating SSRF, XXE, stored XSS, or other out-of-band behavior, enable OAST in "Settings -> Analysis", provide the Worker management endpoint and Worker API key, then run the connection test.',
            'If you need to build or debug the Cloudflare Worker backend, open the separate "Traffic Analysis / OAST" document instead of relying on compressed notes here.',
            'Generate a token and payload from the Worker management endpoint, then use "Insert OAST payload" in Repeater or Intruder or paste the generated callback URL directly into the request before sending it.',
            'Filter hits in the OAST panel by token, domain, source request, or event content, and export the current result set as JSON when you need evidence.',
            'Review tokens, payloads, hit counts, callback events, and the linked source request in the OAST panel; verify Host, URL, IP, user agent, timestamp, and the originating request, and delete stale or noisy callback events when needed.',
            'Once the behavior is reproducible, carry the conclusion back into Security Center, Bug Bounty, or workflows.',
          ],
        },
        enTrafficOastEntry,
        {
          id: 'feature-ai-assistant',
          icon: 'fas fa-brain',
          title: 'AI Assistant',
          route: '/ai-assistant',
          summary:
            'The AI Assistant is the unified conversation workspace for explanation, synthesis, grounded planning, and tool-assisted reasoning, but it does not replace evidence.',
          capabilities: [
            'It is the entry for multi-session assistant work and assistant governance, not only a chat panel.',
            'Consume RAG context, history, and tool integrations.',
            'The detailed guides for sessions/roles and forced rules/turn logs now live in separate help entries in the same directory.',
          ],
          operations: [
            'Open "AI Assistant / Sessions and Roles" when the issue is about context organization or role selection.',
            'Open "AI Assistant / Forced Rules and Turn Logs" when the issue is about governance or one-turn debugging.',
            'Take the suggested next step back into the operational page and verify it there.',
          ],
        },
        enAiAssistantSessionsEntry,
        enAiAssistantGovernanceEntry,
        {
          id: 'feature-workflow-studio',
          icon: 'fas fa-diagram-project',
          title: 'Workflow Studio',
          route: '/workflow-studio',
          summary:
            'Workflow Studio converts repeatable operations into structured flows. It includes workflow lists, templates, canvas editing, parameters, logs, history, import/export, and scheduling.',
          capabilities: [
            'It is the main entry for both workflow design and workflow runtime behavior.',
            'Create, load, clone, delete, and template workflows.',
            'The design-side guide and the execution/history guide now live in separate help entries in the same directory.',
          ],
          operations: [
            'Open "Workflow Studio / Canvas and Parameters" when the issue is about graph design or configuration.',
            'Open "Workflow Studio / Execution and History" when the issue is about runtime, logs, or past runs.',
            'Stabilize the workflow into a template or callable tool only after both sides are working.',
          ],
        },
        enWorkflowCanvasEntry,
        enWorkflowExecutionEntry,
        {
          id: 'feature-bug-bounty',
          icon: 'fas fa-bullseye',
          title: 'Bug Bounty',
          route: '/bug-bounty',
          summary:
            'The Bug Bounty module supports continuous target operations instead of only storing program metadata. It connects programs, assets, APIs, findings, knowledge, submissions, changes, workflows, and monitoring.',
          capabilities: [
            'Provide tabs for programs, assets, API inventory, findings, knowledge, submissions, statistics, import/export, templates, changes, workflows, and monitor.',
            'Track findings and submission outcomes under program scope.',
            'Connect change events, monitoring, and workflow templates into continuous operations.',
          ],
          operations: [
            'Start by selecting or creating a target program and maintaining scope assets.',
            'Record findings, knowledge, and submission actions under the same program context.',
            'When the task narrows to one slice, jump into the Findings, Knowledge, or Continuous Ops help entry in the same directory for the detailed operating model.',
          ],
        },
        enBugBountyFindingsEntry,
        enBugBountyKnowledgeEntry,
        enBugBountyOperationsEntry,
      ],
    },
    {
      id: 'tools-and-content',
      badge: 'Tools',
      title: 'Tools, extensions, and knowledge content',
      description:
        'This group focuses on bringing capability into the platform or persisting capability inside the platform: tools, plugins, dictionaries, knowledge, and data transforms.',
      entries: [
        {
          id: 'feature-mcp-tools',
          icon: 'fas fa-tools',
          title: 'App Tools / MCP Tools',
          route: '/mcp-tools',
          summary:
            'The Tools page is the capability integration layer. It currently exposes built-in tools, MCP servers, and skills, with server details, tool parameters, and config import support.',
          capabilities: [
            'It is the main entry for platform capability integration rather than only a server configuration page.',
            'Show built-in, workflow, and plugin-based tools.',
            'The detailed guides for MCP Servers and Built-ins/Skills now live in separate help entries in the same directory.',
          ],
          operations: [
            'Open "App Tools / MCP Servers" when the task is external capability integration.',
            'Open "App Tools / Built-ins and Skills" when the task is about capability already inside the platform.',
            'Once the boundary is clear and the capability is stable, use it from AI Assistant, workflows, or operational pages.',
          ],
        },
        enMcpServersEntry,
        enBuiltinSkillsEntry,
        {
          id: 'feature-cyberchef',
          icon: 'fas fa-terminal',
          title: 'CyberChef',
          route: '/cyberchef',
          summary:
            'CyberChef is the transformation workspace for encoded data, text reshaping, and temporary data handling during verification and debugging.',
          capabilities: [
            'Handle encode, decode, format, and transform tasks.',
            'Fit payload preparation, artifact normalization, and debugging support.',
            'Remain useful as a standalone utility page during verification work.',
          ],
          operations: [
            'Paste the input into CyberChef.',
            'Apply the required transformation steps and validate the output.',
            'Carry the transformed output back into traffic, plugin, workflow, or reporting tasks.',
          ],
        },
        {
          id: 'feature-dictionary',
          icon: 'fas fa-book',
          title: 'Dictionary Management',
          route: '/dictionary',
          summary:
            'Dictionary Management maintains reusable wordlists, rules, and structured templates for brute force, matching, generation, and automation scenarios.',
          capabilities: [
            'Maintain dictionary entries, rule templates, and structured lists.',
            'Support batch editing and rule-based generation.',
            'Provide reusable inputs for intruder, workflows, plugins, and search-related flows.',
          ],
          operations: [
            'Decide whether the dictionary is meant for lists, rules, or templates.',
            'Create or update entries and batch-edit them when consistency matters.',
            'Reuse the resulting dictionary data in later operational paths instead of rewriting inputs manually.',
          ],
        },
        {
          id: 'feature-plugin-management',
          icon: 'fas fa-puzzle-piece',
          title: 'Plugin Management',
          route: '/plugins',
          summary:
            'Plugin Management owns custom runtime extensions. It covers creation, upload, AI generation, categorization, plugin store, review, testing, enablement, and code inspection.',
          capabilities: [
            'It acts as the plugin control surface that groups lifecycle work, store flows, and review flows.',
            'Filter, favorite, batch-enable, review, delete, and install from the store.',
            'The detailed lifecycle guide and the store/review governance guide now live in separate help entries in the same directory.',
          ],
          operations: [
            'Decide first whether you are dealing with lifecycle work or store/review work.',
            'Open "Plugin Management / Lifecycle" for create, upload, AI generation, testing, and enablement.',
            'Open "Plugin Management / Store and Review" for source governance and runtime admission.',
          ],
        },
        enPluginLifecycleEntry,
        enPluginStoreReviewEntry,
        {
          id: 'feature-rag-management',
          icon: 'fas fa-database',
          title: 'RAG Management',
          route: '/rag-management',
          summary:
            'RAG Management organizes long-lived knowledge for AI consumption. It supports collection stats, filtering, collection creation, ingestion, activation, and query validation.',
          capabilities: [
            'It is the main entry for long-lived AI-readable knowledge rather than only a file upload view.',
            'Manage collection boundaries, activation, ingestion, query validation, and detail inspection.',
            'The collection guide and the ingest/query guide now live as separate help entries in the same directory.',
          ],
          operations: [
            'Open "RAG Management / Collections" when the problem is structural or activation-related.',
            'Open "RAG Management / Ingest and Query" when the problem is document import or recall validation.',
            'After validating collections or query quality, return to AI Assistant and confirm end-to-end usage.',
          ],
        },
        enRagCollectionsEntry,
        enRagIngestQueryEntry,
      ],
    },
    {
      id: 'system-and-collaboration',
      badge: 'System',
      title: 'System configuration, notifications, and coordination',
      description:
        'This group handles platform governance and runtime configuration: agents, settings, notification rules, the message center, and performance visibility.',
      entries: [
        {
          id: 'feature-agent-management',
          icon: 'fas fa-robot',
          title: 'Agent Management',
          route: '/agent-management',
          summary:
            'Agent Management keeps interactive agents, background agents, and their shared runtime environment in one place. The current top-level areas are agent registry and runtime settings.',
          capabilities: [
            'Maintain agent definitions and registration state.',
            'Manage terminal execution environment, working directory, uploads, and timeouts.',
            'Govern how agents run across the platform.',
          ],
          operations: [
            'Open /agent-management and decide whether the change belongs to agent definitions or runtime settings.',
            'Use the agent area for registry-level changes and the runtime area for execution configuration.',
            'Validate the result from AI Assistant or the relevant automation path after the change.',
          ],
        },
        {
          id: 'feature-settings',
          icon: 'fas fa-cog',
          title: 'Settings',
          route: '/settings',
          summary:
            'Settings is the centralized configuration page. It is currently grouped into AI, RAG, Database, System, Network, and Security categories.',
          capabilities: [
            'Configure AI providers, models, tests, defaults, and usage details.',
            'Configure RAG embeddings, chunking, and retrieval behavior.',
            'Configure storage, appearance, network, and security policies in one place.',
          ],
          operations: [
            'Select the correct settings category first instead of scanning the entire page.',
            'Change the relevant values and run the available test or validation action.',
            'Save the configuration and verify its effect from the operational page that depends on it.',
          ],
        },
        {
          id: 'feature-notification-rules',
          icon: 'fas fa-sliders',
          title: 'Notification Rule Management',
          route: '/notifications',
          summary:
            'Notification Rules define which events are delivered, through which channel, and to which endpoint. The page currently supports Feishu, DingTalk, WeCom, Webhook, and Email.',
          capabilities: [
            'Create, edit, enable, disable, and delete notification rules.',
            'Support channel-specific configuration for multiple delivery channels.',
            'Test delivery connectivity before enabling a rule.',
          ],
          operations: [
            'Create a rule and define the notification type plus description.',
            'Choose the delivery channel, fill in credentials or endpoint details, and run a connection test.',
            'Enable the rule only after the test succeeds, then confirm that real events are delivered as intended.',
          ],
        },
        {
          id: 'feature-notification-center',
          icon: 'fas fa-bell',
          title: 'Notification Center',
          route: '/notification-center',
          summary:
            'Notification Center is the history and triage page for asynchronous outputs. It centralizes messages and notifications by category, source, and unread state.',
          capabilities: [
            'Provide All, Messages, and Notifications category views.',
            'Filter by source, unread state, and activity age.',
            'Adjust delivery preferences and jump directly to rule management.',
          ],
          operations: [
            'Filter the center by category and source until only actionable items remain.',
            'Open entries, mark them as read, or clear them after handling.',
            'Move to notification rules when the event routing or delivery path itself needs to change.',
          ],
        },
        {
          id: 'feature-performance-monitor',
          icon: 'fas fa-gauge-high',
          title: 'Performance Monitor',
          route: '/performance',
          summary:
            'Performance Monitor is the dedicated runtime visibility page for identifying load spikes, contention, or abnormal runtime behavior.',
          capabilities: [
            'Offer a separate view for runtime performance observation.',
            'Support diagnosis during heavy traffic, AI, workflow, or plugin activity.',
            'Act as an auxiliary page for performance troubleshooting.',
          ],
          operations: [
            'Open /performance when you suspect lag, congestion, or abnormal resource usage.',
            'Compare the page state with the workflows or tasks currently running elsewhere in the app.',
            'Return to the responsible module and adjust concurrency, configuration, or execution strategy.',
          ],
        },
      ],
    },
  ],
  faqTitle: 'Common decision points',
  faqDescription:
    'When you are unsure where to start, answer these questions first. They usually narrow the correct entry point quickly.',
  faqs: [
    {
      id: 'faq-traffic-before-ai',
      question: 'Why is traffic usually recommended before AI?',
      answer:
        'Because traffic, samples, and context are evidence, while AI is the reasoning layer. Without evidence, AI output is only a hypothesis, not validation.',
    },
    {
      id: 'faq-search-or-help',
      question: 'When should I use Global Search versus the help document?',
      answer:
        'Use Global Search when you know the goal but not the entry point. Use the help document when you already found the page but need to understand responsibility and operating steps.',
    },
    {
      id: 'faq-workflow-or-plugin',
      question: 'How should I distinguish workflows, plugins, and MCP tools?',
      answer:
        'Workflows connect multiple actions into a flow. Plugins define runtime logic inside the platform. MCP tools connect outside capability into the platform.',
    },
    {
      id: 'faq-settings-boundary',
      question: 'Why can some changes be made both from the top bar and from Settings?',
      answer:
        'Top-bar controls are for immediate switching. Settings are for persisted and more granular configuration. One solves “change now,” the other solves “run this way by default.”',
    },
    {
      id: 'faq-oast-not-dnslog',
      question: 'Why does OAST sometimes show no hit or fewer hits than expected?',
      answer:
        'Because a Worker-based OAST backend is an HTTP/HTTPS callback platform, not an authoritative DNSLog. If the target only resolves DNS and never follows with an HTTP/HTTPS request, the Worker sees nothing. Another common cause is incomplete setup: missing oast and *.oast DNS records, proxy disabled, or Worker Routes that cover only the root host or only the wildcard host instead of both.',
    },
    {
      id: 'faq-feature-entitlement-visibility',
      question: 'Why does the help document mention features that are not visible in my sidebar?',
      answer:
        'Because the help document describes capabilities that exist in the product, while actual navigation can still be gated by license, entitlement, or current runtime configuration. Bug Bounty is the clearest example: the document explains its role, but the sidebar only shows the entry when the entitlement is enabled. Check whether the feature is unavailable versus simply not exposed in the current instance.',
    },
  ],
  closingNote:
    'The intended way to use this document is not to read it front to back. Find your current problem category in the left tree first, then follow the operating steps under that feature entry.',
}

export function getHelpCenterContent(locale: string): HelpCenterContent {
  return locale.startsWith('zh') ? zhContent : enContent
}
