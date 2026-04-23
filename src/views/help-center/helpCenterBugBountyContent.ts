import type { HelpCenterFeatureEntry } from './helpCenterContent'

export const zhBugBountyFindingsEntry = {
  id: 'feature-bug-bounty-findings',
  icon: 'fas fa-file-circle-exclamation',
  title: '漏洞赏金 / Findings',
  route: '/bug-bounty -> Findings',
  summary:
    'Findings 是漏洞赏金里最接近落地结果的标签页，负责把项目范围内的发现、证据、状态、优先级和后续提交流转绑定在一起。',
  capabilities: [
    '集中查看当前项目下的发现列表、严重性、状态和更新时间。',
    '把证据、复现说明、资产/API 上下文和提交状态绑定在同一个发现条目上。',
    '适合作为“从线索到提交”之间的统一工作台，而不是把结果散落在外部笔记里。',
  ],
  operations: [
    '先进入对应 Program，再切到 Findings，确认你当前处理的是哪个项目上下文。',
    '把复现结果、请求证据、影响范围和优先级填进发现条目，再决定是否推进到 Submission。',
    '如果同类问题会重复出现，继续把稳定结论沉淀到同目录下的 Knowledge 文档项里。',
  ],
  detailSections: [
    {
      id: 'bounty-findings-role',
      title: '一、Findings 的职责',
      items: [
        'Findings 解决的是“这个问题是否已经成为正式发现”。',
        '它不是临时草稿区，也不是外部提交页，而是项目内的正式结果池。',
        '进入 Submission 之前，建议先在 Findings 里把证据和状态整理干净。',
      ],
    },
    {
      id: 'bounty-findings-fields',
      title: '二、建议至少维护这些信息',
      items: [
        '发现标题、严重性、当前状态。',
        '命中的 Program、Asset、API 或其它范围对象。',
        '复现步骤、关键请求响应、截图或结构化证据。',
        '是否已经提交、提交到哪里、提交后的回执状态。',
      ],
    },
    {
      id: 'bounty-findings-flow',
      title: '三、推荐处理顺序',
      ordered: true,
      items: [
        '先在流量分析里完成复现，确认不是噪音。',
        '在 Findings 中创建或更新条目，把证据挂进去。',
        '根据影响和可信度做严重性与优先级判断。',
        '需要对外提交时，再进入 Submissions 继续推进。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhBugBountyKnowledgeEntry = {
  id: 'feature-bug-bounty-knowledge',
  icon: 'fas fa-book-open-reader',
  title: '漏洞赏金 / Knowledge',
  route: '/bug-bounty -> Knowledge',
  summary:
    'Knowledge 负责沉淀项目特有知识，而不是只存通用漏洞定义。它适合记录目标系统特征、历史规律、命中条件、提交流程和后续复用资料。',
  capabilities: [
    '把项目级知识和发现结果拆开，避免所有上下文都挤在 Findings 里。',
    '记录目标特点、常见入口、已验证假设、提交偏好和历史经验。',
    '方便后续复测、换人接手和把经验继续转进工作流或知识库。',
  ],
  operations: [
    '当某条信息已经超出“单个发现”的边界时，把它放进 Knowledge。',
    '把目标特征、提交规则、常见误报点和可复用 payload 整理成稳定条目。',
    '如果知识已经从项目经验演化成通用资产，再同步到平台级知识库管理中。',
  ],
  detailSections: [
    {
      id: 'bounty-knowledge-what',
      title: '一、什么应该写进 Knowledge',
      items: [
        '目标系统的登录链路、环境差异、接口分层、鉴权规律。',
        '历史上验证过的 payload、特殊 header、速率限制和误报模式。',
        '项目方的提交流程、奖励偏好、修复回访方式。',
      ],
    },
    {
      id: 'bounty-knowledge-boundary',
      title: '二、它和 Findings 的边界',
      items: [
        'Findings 记录的是某一次发现。',
        'Knowledge 记录的是跨多次发现仍然有效的上下文。',
        '同一项目里，Knowledge 负责复用，Findings 负责结案。',
      ],
    },
    {
      id: 'bounty-knowledge-reuse',
      title: '三、推荐沉淀方式',
      ordered: true,
      items: [
        '先把重复出现的结论从 Findings 中抽离出来。',
        '用项目维度整理成可以长期复用的知识条目。',
        '稳定后再决定是否同步到平台级知识库或工作流模板。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhBugBountyOperationsEntry = {
  id: 'feature-bug-bounty-operations',
  icon: 'fas fa-wave-square',
  title: '漏洞赏金 / 持续监控',
  route: '/bug-bounty -> Changes / Workflows / Monitor',
  summary:
    '这部分负责把漏洞赏金从一次性项目变成持续化运营。Changes、Workflows 和 Monitor 解决的是“目标变化怎么被发现、怎么触发、怎么持续跟进”。',
  capabilities: [
    '用 Changes 跟踪范围、资产或接口变化。',
    '用 Workflows 把重复动作变成可执行流程。',
    '用 Monitor 维持长期观察，而不是只做一次扫描后结束。',
  ],
  operations: [
    '当目标面会持续变化时，不要只停留在 Findings，继续接上 Changes、Workflows 和 Monitor。',
    '先定义什么变化值得关注，再用工作流把后续动作串起来。',
    '把高频重复动作收敛成持续运行能力，而不是每次重新手工排查。',
  ],
  detailSections: [
    {
      id: 'bounty-ops-changes',
      title: '一、Changes',
      items: [
        '适合发现资产、接口、页面或项目范围的变化事件。',
        '变化本身不是发现，但它经常是新发现的起点。',
        '先定义监控边界，再决定变化后要触发什么动作。',
      ],
    },
    {
      id: 'bounty-ops-workflows',
      title: '二、Workflows',
      items: [
        '把“收到变化 -> 收集证据 -> 触发测试 -> 记录结果”变成固定流程。',
        '避免每次目标变化都重新拼接人工步骤。',
        '适合把项目专用经验和平台自动化连起来。',
      ],
    },
    {
      id: 'bounty-ops-monitor',
      title: '三、Monitor',
      ordered: true,
      items: [
        '确定需要长期观察的目标、资产或接口。',
        '把变化检测和工作流触发接上。',
        '定期回看产出的变化、发现和提交流转结果，修正监控边界。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enBugBountyFindingsEntry = {
  id: 'feature-bug-bounty-findings',
  icon: 'fas fa-file-circle-exclamation',
  title: 'Bug Bounty / Findings',
  route: '/bug-bounty -> Findings',
  summary:
    'Findings is the result-focused tab in Bug Bounty. It binds evidence, state, severity, scope context, and later submission flow under the same project-facing record.',
  capabilities: [
    'Review findings, severity, status, and update time inside the selected program.',
    'Keep evidence, reproduction notes, scope context, and submission state on the same finding record.',
    'Use it as the handoff point between technical validation and formal submission.',
  ],
  operations: [
    'Open the relevant program first, then switch to Findings so you stay in the correct project scope.',
    'Store the reproduction result, request evidence, impact, and priority before pushing the issue toward submission.',
    'When a conclusion becomes reusable project knowledge instead of a single issue, move that part into the Knowledge doc under the same directory.',
  ],
  detailSections: [
    {
      id: 'bounty-findings-role',
      title: '1. Purpose of Findings',
      items: [
        'Findings answers whether an issue has become a formal tracked result.',
        'It is not only a scratchpad and not yet the external submission surface.',
        'Clean evidence and state in Findings before moving into Submissions.',
      ],
    },
    {
      id: 'bounty-findings-fields',
      title: '2. Minimum Information to Maintain',
      items: [
        'Title, severity, and current state.',
        'Linked program, asset, API, or other scope object.',
        'Reproduction steps, critical request/response evidence, and screenshots if needed.',
        'Submission state, target channel, and any reply status.',
      ],
    },
    {
      id: 'bounty-findings-flow',
      title: '3. Recommended Flow',
      ordered: true,
      items: [
        'Validate the behavior in Traffic Analysis first.',
        'Create or update the finding and attach evidence.',
        'Set severity and priority based on impact and confidence.',
        'Move into Submissions only after the finding is stable enough.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enBugBountyKnowledgeEntry = {
  id: 'feature-bug-bounty-knowledge',
  icon: 'fas fa-book-open-reader',
  title: 'Bug Bounty / Knowledge',
  route: '/bug-bounty -> Knowledge',
  summary:
    'Knowledge stores project-specific context rather than single findings. It is the right place for target patterns, validated assumptions, submission habits, and reusable research notes.',
  capabilities: [
    'Separate reusable program knowledge from one-off findings.',
    'Record target behavior, validated payloads, submission expectations, and recurring false-positive patterns.',
    'Support retesting, handoffs, and later transfer into platform-level knowledge or workflows.',
  ],
  operations: [
    'Move information into Knowledge once it no longer belongs to a single finding.',
    'Organize target-specific context, submission rules, and reusable payload patterns there.',
    'If the content becomes generally useful instead of project-specific, move it further into the platform-level knowledge base.',
  ],
  detailSections: [
    {
      id: 'bounty-knowledge-what',
      title: '1. What Belongs in Knowledge',
      items: [
        'Login flows, environment differences, API layout, and auth rules.',
        'Validated payloads, required headers, rate limits, and recurring false positives.',
        'Program submission habits, reward preferences, and remediation follow-up practices.',
      ],
    },
    {
      id: 'bounty-knowledge-boundary',
      title: '2. Boundary Versus Findings',
      items: [
        'Findings stores one concrete issue.',
        'Knowledge stores context that survives across multiple issues.',
        'Within the same program, Knowledge is for reuse while Findings is for case handling.',
      ],
    },
    {
      id: 'bounty-knowledge-reuse',
      title: '3. Recommended Reuse Path',
      ordered: true,
      items: [
        'Pull repeated conclusions out of individual findings.',
        'Rewrite them as reusable program-level knowledge.',
        'Promote them into the global knowledge base or workflow templates only after they become stable enough.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enBugBountyOperationsEntry = {
  id: 'feature-bug-bounty-operations',
  icon: 'fas fa-wave-square',
  title: 'Bug Bounty / Continuous Ops',
  route: '/bug-bounty -> Changes / Workflows / Monitor',
  summary:
    'This area turns Bug Bounty from one-off project handling into continuous operations. Changes, Workflows, and Monitor define how target movement is detected, triggered, and followed over time.',
  capabilities: [
    'Use Changes to observe scope, asset, and API movement.',
    'Use Workflows to connect repeated actions into automation.',
    'Use Monitor to keep long-running visibility instead of ending after a single pass.',
  ],
  operations: [
    'When the target changes over time, do not stop at Findings. Continue into Changes, Workflows, and Monitor.',
    'Define which changes matter before wiring follow-up actions.',
    'Turn repeated project work into continuous capability rather than repeating manual review each time.',
  ],
  detailSections: [
    {
      id: 'bounty-ops-changes',
      title: '1. Changes',
      items: [
        'Track scope, asset, interface, or page changes over time.',
        'A change event is not yet a finding, but it is often the starting signal.',
        'Define the monitoring boundary before deciding what should happen next.',
      ],
    },
    {
      id: 'bounty-ops-workflows',
      title: '2. Workflows',
      items: [
        'Convert “change detected -> collect evidence -> trigger testing -> record result” into a stable flow.',
        'Avoid rebuilding the same manual sequence whenever the target changes.',
        'Connect project-specific knowledge with platform automation.',
      ],
    },
    {
      id: 'bounty-ops-monitor',
      title: '3. Monitor',
      ordered: true,
      items: [
        'Choose the assets or interfaces that require long-term observation.',
        'Connect change detection with workflow execution.',
        'Review the resulting changes, findings, and submission outcomes regularly and adjust the boundary.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry
