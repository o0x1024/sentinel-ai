import type { HelpCenterFeatureEntry } from './helpCenterContent'

export const zhAiAssistantSessionsEntry = {
  id: 'feature-ai-assistant-sessions',
  icon: 'fas fa-comments',
  title: 'AI 助手 / 会话与角色',
  route: '/ai-assistant -> 会话标签 / 角色选择',
  summary:
    '这部分只讲 AI 助手工作区本身：多会话标签、当前会话切换、角色选择，以及把证据和上下文送进正确会话的方式。',
  capabilities: [
    '支持多会话标签，不同任务可以并行保留在不同上下文里。',
    '支持默认助手和自定义角色切换，角色决定对话时的职责和风格边界。',
    '支持把流量和资产引用注入当前会话，而不是只靠手工复述上下文。',
  ],
  operations: [
    '先判断当前任务应该进入已有会话还是新标签页，不要把无关任务混进一个上下文。',
    '再根据目标切换默认助手或自定义角色。',
    '提交前尽量把证据、引用和背景带全，减少 AI 只凭猜测回答。',
  ],
  detailSections: [
    {
      id: 'ai-sessions-tabs',
      title: '一、会话标签怎么用',
      items: [
        '标签页是上下文边界，不是视觉分组。',
        '不同目标、不同案件、不同排查链路，最好拆到不同会话。',
        '如果同一会话已经承载太多无关任务，下一步建议新开标签页而不是继续堆上下文。',
      ],
    },
    {
      id: 'ai-sessions-roles',
      title: '二、角色选择解决什么',
      items: [
        '默认助手适合通用解释、总结和排查建议。',
        '自定义角色适合把某一类固定任务的提示词和工作边界提前固化。',
        '切换角色不是为了换语气，而是为了改变任务约束和关注重点。',
      ],
    },
    {
      id: 'ai-sessions-context',
      title: '三、推荐顺序',
      ordered: true,
      items: [
        '选会话或新建标签页。',
        '选默认助手或自定义角色。',
        '补充证据、流量引用、资产引用和背景。',
        '再提交问题，避免让 AI 在上下文不全时直接推理。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhAiAssistantGovernanceEntry = {
  id: 'feature-ai-assistant-governance',
  icon: 'fas fa-file-signature',
  title: 'AI 助手 / 强制规则与 Turn 日志',
  route: '/ai-assistant -> 强制规则 / Turn 日志',
  summary:
    '这一部分处理 AI 助手的运行约束和可审计性。强制规则决定对话必须遵守什么边界，Turn 日志负责回看单轮执行发生了什么。',
  capabilities: [
    '支持管理用户级强制规则，给所有对话施加统一约束。',
    '支持查看 Turn 日志，回看某轮执行的详细过程。',
    '适合回答“为什么这轮回答是这样”“为什么这轮没有按预期执行”。',
  ],
  operations: [
    '如果问题是行为边界不稳定，先看强制规则。',
    '如果问题是单轮执行异常、结果偏差或过程不可解释，先看 Turn 日志。',
    '不要把这两块当普通 UI 功能，它们是 AI 助手的治理层。',
  ],
  detailSections: [
    {
      id: 'ai-governance-rules',
      title: '一、强制规则的职责',
      items: [
        '它为所有会话提供统一约束，而不是单次提示词补丁。',
        '适合沉淀必须遵守的行为规则和输出边界。',
        '如果你反复在每次提问里写相同约束，说明它更适合进入强制规则。',
      ],
    },
    {
      id: 'ai-governance-logs',
      title: '二、Turn 日志适合查什么',
      items: [
        '查看某轮执行到底做了哪些动作。',
        '回看为什么结果异常、为什么没引用到预期上下文。',
        '把日志当作排查层，而不是普通聊天历史替代品。',
      ],
    },
    {
      id: 'ai-governance-boundary',
      title: '三、边界',
      items: [
        '会话与角色解决“当前任务怎么组织”。',
        '强制规则解决“所有任务必须遵守什么边界”。',
        'Turn 日志解决“某一轮到底发生了什么”。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhMcpServersEntry = {
  id: 'feature-mcp-servers',
  icon: 'fas fa-server',
  title: '应用工具 / MCP Servers',
  route: '/mcp-tools -> MCP Servers',
  summary:
    '这部分只讲外部能力接入：MCP Server 的新增、JSON 导入、详情查看、配置编辑、连接测试和工具参数查看。',
  capabilities: [
    '支持通过 JSON 导入 MCP Server 配置，而不是逐字段手工拼接。',
    '支持查看 server 详情、编辑 transport、endpoint、headers、command 与 args。',
    '支持查看服务器暴露出的工具及其参数约束。',
  ],
  operations: [
    '如果目标是接入外部能力，直接进入 MCP Servers，而不是先去内置工具或 Skills。',
    '先把 server 配通，再去看它暴露出的 tools，不要顺序反过来。',
    '参数约束看不清时，进入详情里的 tools 标签逐项确认 schema。',
  ],
  detailSections: [
    {
      id: 'mcp-servers-config',
      title: '一、先解决 server 配置',
      items: [
        'MCP 接入先看 transport、endpoint 或 command/args 是否正确。',
        '远程 server 还要确认 headers、认证和连接方式。',
        '如果 server 本身没接通，后面工具列表和调用都没有意义。',
      ],
    },
    {
      id: 'mcp-servers-tools',
      title: '二、再看工具 schema',
      items: [
        'Server 详情里的 tools 标签用于看暴露出的工具和参数要求。',
        '参数约束不清时，直接看 schema，而不是靠猜。',
        '这一步决定你后面在 AI 助手或工作流里能不能正确调用工具。',
      ],
    },
    {
      id: 'mcp-servers-import',
      title: '三、推荐顺序',
      ordered: true,
      items: [
        '新增 server 或粘贴 JSON 配置。',
        '确认 transport、命令或 endpoint。',
        '连接成功后进入详情查看 tools schema。',
        '再回到 AI 助手或工作流里消费这些工具。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhBuiltinSkillsEntry = {
  id: 'feature-builtin-skills',
  icon: 'fas fa-toolbox',
  title: '应用工具 / 内置工具与 Skills',
  route: '/mcp-tools -> Builtin Tools / Skills',
  summary:
    '这部分处理平台内已有能力，而不是外部 server。Builtin Tools 视图负责看内置、workflow、plugin 三类工具来源；Skills 视图负责看技能层能力。',
  capabilities: [
    'Builtin Tools 支持按 builtin、workflow、plugin 三类来源切换。',
    'Workflow Tools 和 Plugin Tools 让你看到平台内部衍生出来的工具能力。',
    'Skills 视图负责管理技能层能力，而不是 server 连接本身。',
  ],
  operations: [
    '如果你要看平台里已经有哪些工具可用，先从 Builtin Tools 开始。',
    '如果你要看 workflow 或 plugin 暴露出来的工具能力，就切来源视图，不要误以为它们属于 MCP Servers。',
    '如果目标是技能层管理，直接进入 Skills 标签。',
  ],
  detailSections: [
    {
      id: 'builtin-sources',
      title: '一、内置工具来源怎么区分',
      items: [
        'builtin：平台原生工具。',
        'workflow：由工作流暴露出来的工具能力。',
        'plugin：由插件提供的工具能力。',
      ],
    },
    {
      id: 'builtin-skills-boundary',
      title: '二、Builtin / MCP / Skills 的边界',
      items: [
        'Builtin Tools 看平台内部已有能力。',
        'MCP Servers 看外部接入能力。',
        'Skills 看技能层组织，不直接替代 server 配置。',
      ],
    },
    {
      id: 'builtin-skills-flow',
      title: '三、推荐顺序',
      ordered: true,
      items: [
        '先判断能力来自平台内部还是外部 server。',
        '内部能力先看 Builtin Tools 或 Skills。',
        '外部能力再去 MCP Servers。',
        '确认能力边界后，再回 AI 助手或工作流实际调用。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enAiAssistantSessionsEntry = {
  id: 'feature-ai-assistant-sessions',
  icon: 'fas fa-comments',
  title: 'AI Assistant / Sessions and Roles',
  route: '/ai-assistant -> Tabs / Roles',
  summary:
    'This entry focuses on the assistant workspace itself: multi-session tabs, active conversation switching, role selection, and sending the right evidence into the right conversation.',
  capabilities: [
    'Support multiple session tabs so unrelated tasks keep separate context.',
    'Support the default assistant and custom roles that change task framing and constraints.',
    'Support injecting traffic and asset references instead of relying only on manual summary text.',
  ],
  operations: [
    'Decide first whether the task belongs in an existing conversation or a new tab.',
    'Choose the default assistant or a custom role based on the job.',
    'Attach evidence and references before asking the model to reason.',
  ],
  detailSections: [
    {
      id: 'ai-sessions-tabs',
      title: '1. How to Use Session Tabs',
      items: [
        'Tabs are context boundaries, not only visual organization.',
        'Different cases, goals, or investigation chains should usually live in different sessions.',
        'If one session already carries too many unrelated tasks, start a new tab instead of piling on more context.',
      ],
    },
    {
      id: 'ai-sessions-roles',
      title: '2. What Role Selection Solves',
      items: [
        'The default assistant fits general explanation, summarization, and planning.',
        'Custom roles fit fixed task types with stable instructions and boundaries.',
        'The point of switching roles is changing constraints and focus, not only wording style.',
      ],
    },
    {
      id: 'ai-sessions-context',
      title: '3. Recommended Sequence',
      ordered: true,
      items: [
        'Choose the session or open a new tab.',
        'Choose the default assistant or a custom role.',
        'Attach evidence, traffic references, asset references, and context.',
        'Then submit the question so the model does not have to guess.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enAiAssistantGovernanceEntry = {
  id: 'feature-ai-assistant-governance',
  icon: 'fas fa-file-signature',
  title: 'AI Assistant / Forced Rules and Turn Logs',
  route: '/ai-assistant -> Forced Rules / Turn Logs',
  summary:
    'This entry covers assistant governance and auditability. Forced rules define what every conversation must obey, and turn logs explain what happened during one execution turn.',
  capabilities: [
    'Manage user-level forced rules as global assistant constraints.',
    'Inspect turn logs for per-turn execution detail.',
    'Answer why a turn behaved unexpectedly or why the result drifted from expectations.',
  ],
  operations: [
    'Check forced rules first when behavior boundaries are unstable.',
    'Check turn logs first when a specific execution turn looks wrong or unexplained.',
    'Treat these as governance tools, not as ordinary UI extras.',
  ],
  detailSections: [
    {
      id: 'ai-governance-rules',
      title: '1. What Forced Rules Do',
      items: [
        'They apply cross-session constraints rather than one-off prompt patches.',
        'They fit rules that must be enforced across all conversations.',
        'If you keep repeating the same instruction in every prompt, it probably belongs here.',
      ],
    },
    {
      id: 'ai-governance-logs',
      title: '2. What Turn Logs Are For',
      items: [
        'Review what one turn actually did.',
        'Investigate why results drifted or why context was not used as expected.',
        'Use logs as a debugging layer rather than as a replacement for chat history.',
      ],
    },
    {
      id: 'ai-governance-boundary',
      title: '3. Boundary',
      items: [
        'Sessions and roles organize the current task.',
        'Forced rules define what every task must obey.',
        'Turn logs explain what happened in one specific turn.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enMcpServersEntry = {
  id: 'feature-mcp-servers',
  icon: 'fas fa-server',
  title: 'App Tools / MCP Servers',
  route: '/mcp-tools -> MCP Servers',
  summary:
    'This entry covers external capability integration: MCP server creation, JSON import, details, configuration editing, connectivity, and tool schema inspection.',
  capabilities: [
    'Support JSON-based MCP server import instead of only field-by-field entry.',
    'Support editing transport, endpoint, headers, command, and args.',
    'Support reviewing exposed tools and their input constraints.',
  ],
  operations: [
    'Go to MCP Servers directly when the task is external capability integration.',
    'Get the server configuration right before worrying about its tools.',
    'Open the tools tab in server details whenever parameter constraints are unclear.',
  ],
  detailSections: [
    {
      id: 'mcp-servers-config',
      title: '1. Solve Server Configuration First',
      items: [
        'Start with transport, endpoint, or command/args.',
        'Remote servers also require correct headers, auth, and connection mode.',
        'If the server is not connected, the later tool view does not matter yet.',
      ],
    },
    {
      id: 'mcp-servers-tools',
      title: '2. Then Inspect Tool Schemas',
      items: [
        'The tools tab in server details shows exposed tools and parameter expectations.',
        'When constraints are unclear, read the schema instead of guessing.',
        'This determines whether later AI Assistant or workflow calls will be valid.',
      ],
    },
    {
      id: 'mcp-servers-import',
      title: '3. Recommended Sequence',
      ordered: true,
      items: [
        'Add the server or paste its JSON config.',
        'Validate transport, command, or endpoint.',
        'After connection succeeds, inspect the tool schemas.',
        'Only then move back into AI Assistant or workflows to use those tools.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enBuiltinSkillsEntry = {
  id: 'feature-builtin-skills',
  icon: 'fas fa-toolbox',
  title: 'App Tools / Built-ins and Skills',
  route: '/mcp-tools -> Builtin Tools / Skills',
  summary:
    'This entry covers capability that already exists inside the platform. The built-in tools view shows builtin, workflow, and plugin sources, while the skills view manages the skill layer.',
  capabilities: [
    'Builtin Tools switches between builtin, workflow, and plugin tool sources.',
    'Workflow Tools and Plugin Tools expose internal platform-derived capability.',
    'The Skills tab manages the skill layer instead of server connectivity.',
  ],
  operations: [
    'Use Builtin Tools first when the capability already lives inside the platform.',
    'Switch source views when the capability comes from workflows or plugins rather than builtin tools.',
    'Use the Skills tab when the task belongs to the skill layer rather than external integration.',
  ],
  detailSections: [
    {
      id: 'builtin-sources',
      title: '1. Distinguish Builtin Tool Sources',
      items: [
        'builtin: native platform tools.',
        'workflow: tools exposed from workflows.',
        'plugin: tools exposed from plugins.',
      ],
    },
    {
      id: 'builtin-skills-boundary',
      title: '2. Builtin vs MCP vs Skills',
      items: [
        'Builtin Tools shows capability already inside the platform.',
        'MCP Servers shows externally connected capability.',
        'Skills organizes the skill layer and does not replace server setup.',
      ],
    },
    {
      id: 'builtin-skills-flow',
      title: '3. Recommended Sequence',
      ordered: true,
      items: [
        'Decide whether the capability is internal or external.',
        'For internal capability, check Builtin Tools or Skills.',
        'For external capability, go to MCP Servers.',
        'After the boundary is clear, return to AI Assistant or workflows for actual usage.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry
