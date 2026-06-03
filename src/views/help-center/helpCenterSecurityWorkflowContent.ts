import type { HelpCenterFeatureEntry } from './helpCenterContent'

export const zhSecurityWorkbenchEntry = {
  id: 'feature-security-workbench',
  icon: 'fas fa-briefcase',
  title: '安全中心 / 工作台',
  route: '/security-center/workbench',
  summary:
    '工作台负责把案件、证据链、对象分析、复核意见和验证动作集中在一个调查视图里。它解决的是“围绕一个问题如何持续推进”而不是“看所有漏洞列表”。',
  capabilities: [
    '围绕 case 展示上下文、证据链、对象分析、复核备注和验证面板。',
    '适合处理还在调查中的问题，而不是已经标准化成纯列表项的问题。',
    '支持在安全中心内完成从线索、证据到复核判断的连续处理。',
  ],
  operations: [
    '当你需要围绕单个案件持续推进时，先进入工作台，而不是直接跳漏洞列表。',
    '先看 case 概览和证据链，再决定是否要做对象分析、补充验证或写复核备注。',
    '工作台里形成稳定判断后，再决定是否回到流量分析复现，或进入漏洞列表做状态流转。',
  ],
  detailSections: [
    {
      id: 'security-workbench-role',
      title: '一、工作台的职责',
      items: [
        '工作台是案件视角，不是结果池视角。',
        '它更适合“问题还在调查中”的阶段。',
        '如果你已经只剩批量改状态、筛选严重性这类动作，应该转去漏洞页。',
      ],
    },
    {
      id: 'security-workbench-panels',
      title: '二、主要内容构成',
      items: [
        'Case 概览：先看案件当前状态和主要上下文。',
        'Evidence Chain：把证据链按顺序串起来，避免判断只停留在单点证据。',
        'Object Analysis / Verification / Review Notes：围绕对象分析、验证和人工复核继续推进。',
      ],
    },
    {
      id: 'security-workbench-flow',
      title: '三、推荐顺序',
      ordered: true,
      items: [
        '先从工作台列表选中当前案件。',
        '看概览和证据链，确认问题是不是还成立。',
        '补充验证、对象分析或复核备注。',
        '需要更深复现时回流量分析；需要正式结果管理时转到漏洞页。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhSecurityFindingsEntry = {
  id: 'feature-security-findings',
  icon: 'fas fa-shield-halved',
  title: '安全中心 / 漏洞与 LLM 安全',
  route: '/vulnerabilities /security-center?tab=llmSecurity',
  summary:
    '这一部分处理标准化结果而不是调查过程。漏洞页适合筛选、详情查看和状态流转；LLM 安全页适合查看相关安全分析结果和运行状态。',
  capabilities: [
    '漏洞页负责列表筛选、详情查看、证据回看和状态管理。',
    'LLM 安全页负责查看相关分析结果，并在导航中体现未读和运行状态。',
    '适合把工作台里收敛过的问题转成正式结果管理。',
  ],
  operations: [
    '当任务已经变成“看结果、改状态、做优先级管理”时，切换到漏洞页。',
    '需要看 LLM 安全相关结果时，切到 LLM 安全标签，不要和普通漏洞列表混看。',
    '如果结果仍需更深复现，再回流量分析；如果需要案件上下文，再回工作台。',
  ],
  detailSections: [
    {
      id: 'security-findings-vulns',
      title: '一、漏洞页适合什么',
      items: [
        '批量筛选和排序。',
        '打开条目看详情、时间线和证据。',
        '做状态流转、优先级和人工复核。',
      ],
    },
    {
      id: 'security-findings-llm',
      title: '二、LLM 安全页适合什么',
      items: [
        '查看 LLM 安全测试或分析结果。',
        '结合未读状态和运行状态快速判断是否有新结果需要处理。',
        '不要把它当普通漏洞页的替代，它是另一类结果视图。',
      ],
    },
    {
      id: 'security-findings-boundary',
      title: '三、边界',
      items: [
        '工作台解决调查推进。',
        '漏洞页和 LLM 安全页解决标准化结果处理。',
        '流量分析解决证据复现。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhWorkflowCanvasEntry = {
  id: 'feature-workflow-canvas',
  icon: 'fas fa-bezier-curve',
  title: '工作流 / 画布与参数',
  route: '/workflow-studio -> 画布 / 参数抽屉 / 元数据',
  summary:
    '这部分只讲工作流设计阶段：列表、模板、节点画布、连线、边映射、参数抽屉和元数据。它解决的是“流程怎么定义”，不是“流程怎么回看执行结果”。',
  capabilities: [
    '支持工作流列表、模板切换、节点画布和侧边节点目录。',
    '支持节点参数抽屉、JSON 校验、工具选择和边映射。',
    '支持名称、描述、标签、版本以及“作为 AI 工具”这类元数据设置。',
  ],
  operations: [
    '先确定从空白工作流、已有工作流还是模板开始。',
    '在画布上加节点、连边、填参数，再做元数据整理。',
    '画布能通过校验后，再进入执行、日志和历史阶段。',
  ],
  detailSections: [
    {
      id: 'workflow-canvas-layout',
      title: '一、设计区由什么组成',
      items: [
        '左侧工作流列表/模板面板：负责加载、克隆、删除、另存模板。',
        '中间画布：负责节点布局、连接关系和可视化。',
        '参数抽屉与边映射：负责节点参数和节点之间的数据传递关系。',
      ],
    },
    {
      id: 'workflow-canvas-params',
      title: '二、参数与边映射',
      items: [
        '节点参数抽屉不是附属功能，它决定节点能不能真实运行。',
        '对象类参数要过 JSON 校验，边映射决定上下游数据怎么传递。',
        '如果参数定义不清，后面的执行日志再详细也只是在放大错误输入。',
      ],
    },
    {
      id: 'workflow-canvas-flow',
      title: '三、推荐顺序',
      ordered: true,
      items: [
        '先加载或新建工作流。',
        '在画布中放节点、连边。',
        '编辑参数、工具选择和边映射。',
        '补元数据并保存。',
        '确认无明显校验错误后再运行。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhWorkflowExecutionEntry = {
  id: 'feature-workflow-execution',
  icon: 'fas fa-play-circle',
  title: '工作流 / 执行与历史',
  route: '/workflow-studio -> 运行 / 日志 / 执行历史',
  summary:
    '这一部分只讲工作流运行后的行为：立即执行、停止、调度、日志、结果面板和执行历史。它解决的是“流程是否按预期跑起来”。',
  capabilities: [
    '支持立即运行、停止运行、启动/停止调度。',
    '支持执行日志、步骤结果面板和详细执行记录。',
    '支持执行历史搜索、查看详情和删除历史记录。',
  ],
  operations: [
    '先运行当前工作流，再结合日志和步骤结果判断问题发生在哪一层。',
    '如果是参数或连线问题，回到画布；如果只是结果回看问题，继续留在历史面板。',
    '当流程需要持续运行时，再启用调度，而不是在未验证前就直接定时化。',
  ],
  detailSections: [
    {
      id: 'workflow-execution-live',
      title: '一、运行中的即时视图',
      items: [
        '顶部按钮负责开始、停止和调度控制。',
        '日志面板适合看当前运行过程中的实时反馈。',
        '结果面板适合看某一步输出，而不是通篇翻日志。',
      ],
    },
    {
      id: 'workflow-execution-history',
      title: '二、执行历史',
      items: [
        '历史面板适合回看过去运行，而不是替代实时日志。',
        '详情对话框会给出步骤级状态、耗时、错误和结果。',
        '如果你要判断流程是否稳定，应该优先看历史而不是只看单次运行。',
      ],
    },
    {
      id: 'workflow-execution-boundary',
      title: '三、边界',
      items: [
        '画布与参数解决“流程怎么定义”。',
        '执行与历史解决“流程有没有按定义跑起来”。',
        '如果日志暴露的是设计问题，立即回设计区，不要在执行页硬扛。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enSecurityWorkbenchEntry = {
  id: 'feature-security-workbench',
  icon: 'fas fa-briefcase',
  title: 'Security Center / Workbench',
  route: '/security-center/workbench',
  summary:
    'The workbench is the case-driven investigation surface. It keeps case context, evidence chains, object analysis, review notes, and verification activity in one place.',
  capabilities: [
    'Organize case context, evidence chains, object analysis, review notes, and verification panels.',
    'Fit investigations that are still evolving instead of already-normalized result handling.',
    'Keep continuous investigation work inside a single security view.',
  ],
  operations: [
    'Open the workbench when the task still revolves around one case rather than a result list.',
    'Review case overview and evidence first, then decide whether more analysis, verification, or review notes are needed.',
    'Move back to Traffic Analysis for deeper reproduction or into the findings view for formal result handling.',
  ],
  detailSections: [
    {
      id: 'security-workbench-role',
      title: '1. What the Workbench Does',
      items: [
        'The workbench is case-oriented rather than list-oriented.',
        'It is best while the issue is still being investigated.',
        'When the task becomes mostly filtering and state updates, move to the findings view.',
      ],
    },
    {
      id: 'security-workbench-panels',
      title: '2. Main Building Blocks',
      items: [
        'Case overview for the current state and top-level context.',
        'Evidence chain for following the reasoning path instead of relying on one isolated artifact.',
        'Object analysis, verification, and review notes for deeper manual progression.',
      ],
    },
    {
      id: 'security-workbench-flow',
      title: '3. Recommended Sequence',
      ordered: true,
      items: [
        'Select the case.',
        'Review overview and evidence chain.',
        'Add verification, object analysis, or review notes.',
        'Return to Traffic Analysis for reproduction or to findings for formal result management.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enSecurityFindingsEntry = {
  id: 'feature-security-findings',
  icon: 'fas fa-shield-halved',
  title: 'Security Center / Findings and LLM Security',
  route: '/vulnerabilities /security-center?tab=llmSecurity',
  summary:
    'This area handles standardized results rather than ongoing investigations. The findings view is for filtering and state transitions; the LLM security view is for its own result stream and status tracking.',
  capabilities: [
    'Use the findings view for filtering, detail review, evidence, and state handling.',
    'Use the LLM security view for its dedicated analysis results and run state.',
    'Turn workbench conclusions into formal result management.',
  ],
  operations: [
    'Switch to the findings view when the task is already result-oriented.',
    'Switch to LLM security when the result belongs to that analysis stream rather than the general finding list.',
    'Move back to Traffic Analysis for reproduction or back to the workbench for richer case context.',
  ],
  detailSections: [
    {
      id: 'security-findings-vulns',
      title: '1. What the Findings View Is For',
      items: [
        'Bulk filtering and sorting.',
        'Opening details, timelines, and evidence.',
        'State updates, prioritization, and manual review.',
      ],
    },
    {
      id: 'security-findings-llm',
      title: '2. What the LLM Security View Is For',
      items: [
        'Viewing LLM-focused security results.',
        'Using unread and running state as triage signals.',
        'Keeping that result stream separate from the general finding list.',
      ],
    },
    {
      id: 'security-findings-boundary',
      title: '3. Boundary',
      items: [
        'Workbench is for investigation progression.',
        'Findings and LLM security are for standardized result handling.',
        'Traffic Analysis is still the reproduction layer.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enWorkflowCanvasEntry = {
  id: 'feature-workflow-canvas',
  icon: 'fas fa-bezier-curve',
  title: 'Workflow Studio / Canvas and Parameters',
  route: '/workflow-studio -> Canvas / Parameters / Metadata',
  summary:
    'This entry focuses on workflow design: lists, templates, canvas editing, edge mapping, parameter drawers, and metadata. It answers how the flow is defined, not how past runs are reviewed.',
  capabilities: [
    'Cover workflow lists, templates, node canvas, and the node catalog sidebar.',
    'Cover parameter drawers, JSON validation, tool selection, and edge mapping.',
    'Cover metadata such as name, description, tags, version, and exposing the workflow as an AI tool.',
  ],
  operations: [
    'Choose whether to start from a blank workflow, an existing flow, or a template.',
    'Add nodes, connect edges, edit parameters, and finalize metadata.',
    'Run only after the graph and parameters are in a valid state.',
  ],
  detailSections: [
    {
      id: 'workflow-canvas-layout',
      title: '1. Main Design Surfaces',
      items: [
        'The workflow list/template panel loads, clones, deletes, and saves templates.',
        'The central canvas defines node placement and flow structure.',
        'Parameter drawers and edge mapping define configuration and data transfer between nodes.',
      ],
    },
    {
      id: 'workflow-canvas-params',
      title: '2. Parameters and Edge Mapping',
      items: [
        'The parameter drawer is part of execution correctness, not only a convenience panel.',
        'Object parameters require JSON validation, and edge mapping controls upstream/downstream data flow.',
        'If parameter design is weak, later runtime logs only amplify bad input.',
      ],
    },
    {
      id: 'workflow-canvas-flow',
      title: '3. Recommended Sequence',
      ordered: true,
      items: [
        'Load or create the workflow.',
        'Place nodes and connect edges.',
        'Edit parameters, tool selection, and edge mapping.',
        'Complete metadata and save.',
        'Run only after obvious validation problems are gone.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enWorkflowExecutionEntry = {
  id: 'feature-workflow-execution',
  icon: 'fas fa-play-circle',
  title: 'Workflow Studio / Execution and History',
  route: '/workflow-studio -> Run / Logs / History',
  summary:
    'This entry focuses on what happens after a workflow is designed: immediate runs, stop actions, scheduling, logs, result panels, and execution history.',
  capabilities: [
    'Run immediately, stop a run, and start or stop schedules.',
    'Inspect execution logs, per-step result panels, and detailed execution records.',
    'Search history, inspect details, and delete past execution records.',
  ],
  operations: [
    'Run first, then use logs and step results to locate where the flow breaks.',
    'Return to the canvas for design problems, but stay in history when the problem is only retrospective inspection.',
    'Enable scheduling only after the flow has already been validated manually.',
  ],
  detailSections: [
    {
      id: 'workflow-execution-live',
      title: '1. Live Execution Surface',
      items: [
        'The header controls start, stop, and scheduling.',
        'The log panel is for live feedback while the workflow is running.',
        'The result panel is better for one step output than scanning the entire log stream.',
      ],
    },
    {
      id: 'workflow-execution-history',
      title: '2. Execution History',
      items: [
        'History is for reviewing previous runs, not replacing live logs.',
        'The detail dialog exposes step-level state, duration, errors, and results.',
        'Use history to judge workflow stability instead of relying on one run.',
      ],
    },
    {
      id: 'workflow-execution-boundary',
      title: '3. Boundary',
      items: [
        'Canvas and parameters define the workflow.',
        'Execution and history verify whether it ran according to that definition.',
        'When logs expose a design flaw, go back to the design surface immediately.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry
