import type { HelpCenterFeatureEntry } from './helpCenterContent'

export const zhPluginLifecycleEntry = {
  id: 'feature-plugin-lifecycle',
  icon: 'fas fa-plug-circle-plus',
  title: '插件管理 / 生命周期',
  route: '/plugins -> 新增 / 上传 / AI 生成 / 启停 / 测试',
  summary:
    '这份文档只讲插件从创建到启用的主线，不再把创建、上传、AI 生成、测试和启停混在插件总览里。',
  capabilities: [
    '覆盖新增插件、上传插件、AI 生成插件三种进入方式。',
    '覆盖常规测试、高级测试、代码查看和启停验证。',
    '强调“启用插件”不等于“插件已经参与目标流程”，必须回到业务页验证。',
  ],
  operations: [
    '先决定插件来源：新建、上传还是 AI 生成。',
    '完成基础信息和代码后，先测试，再启用，不要反过来。',
    '启用后回到流量分析、工作流或智能体等业务页确认插件真实生效。',
  ],
  detailSections: [
    {
      id: 'plugin-lifecycle-entry',
      title: '一、进入方式怎么选',
      items: [
        '新增插件：适合你已经知道要写什么逻辑，直接从空白开始。',
        '上传插件：适合已有现成插件包，需要接入现有能力。',
        'AI 生成插件：适合先快速起草，再人工检查和收敛。',
      ],
    },
    {
      id: 'plugin-lifecycle-validate',
      title: '二、先测再启用',
      items: [
        '常规测试适合快速验证插件是否能正常运行。',
        '高级测试适合检查更贴近真实输入的运行结果。',
        '代码查看和验证报告用于确认 AI 生成或外部上传的逻辑没有明显偏差。',
      ],
    },
    {
      id: 'plugin-lifecycle-finish',
      title: '三、完成闭环',
      ordered: true,
      items: [
        '创建或导入插件。',
        '查看配置和代码，确认插件类型、分类和主要逻辑。',
        '执行测试，修正明显问题。',
        '启用插件。',
        '回到目标业务页确认插件真的参与执行。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhPluginStoreReviewEntry = {
  id: 'feature-plugin-store-review',
  icon: 'fas fa-store',
  title: '插件管理 / 商店与审核',
  route: '/plugins -> 插件商店 / 插件审核',
  summary:
    '这部分只处理来源治理和发布治理：从商店安装、更新插件，以及对待审核插件做批准、拒绝和代码复核。',
  capabilities: [
    '插件商店支持列表/卡片视图、搜索、分类过滤、安装、更新和详情查看。',
    '插件审核支持待审核插件筛选、批量批准/拒绝和详情检查。',
    '适合区分“拿到插件”和“允许插件进入运行环境”这两个动作。',
  ],
  operations: [
    '如果插件来自外部市场，先走插件商店。',
    '如果插件需要进入受控环境，先走审核再决定是否批准。',
    '审核通过也不等于可直接信任，仍然要做测试和业务页验证。',
  ],
  detailSections: [
    {
      id: 'plugin-store-what',
      title: '一、插件商店负责什么',
      items: [
        '拉取商店插件列表并按分类过滤。',
        '查看版本、作者、标签、本地版本与可更新状态。',
        '安装新插件或更新已安装插件。',
      ],
    },
    {
      id: 'plugin-review-what',
      title: '二、插件审核负责什么',
      items: [
        '查看待审核插件的状态、搜索条件和分页结果。',
        '进入详情检查代码和元数据。',
        '对单个或批量插件执行批准或拒绝。',
      ],
    },
    {
      id: 'plugin-store-review-boundary',
      title: '三、边界',
      items: [
        '商店解决来源和安装问题。',
        '审核解决是否允许进入运行环境的问题。',
        '测试解决插件在目标场景里是否真的能工作的问题。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhRagCollectionsEntry = {
  id: 'feature-rag-collections',
  icon: 'fas fa-layer-group',
  title: '知识库管理 / 集合管理',
  route: '/rag-management -> Collections',
  summary:
    '集合管理负责知识的组织边界。它决定资料按什么主题拆分、哪些集合处于激活状态，以及哪些集合只作为后台存储存在。',
  capabilities: [
    '覆盖集合创建、编辑、删除、搜索、状态过滤和统计查看。',
    '支持激活/停用集合，用来控制哪些知识会进入 AI 可消费范围。',
    '包含 agent_memory 这类特殊集合的显示与只读边界。',
  ],
  operations: [
    '先确定集合按项目、SOP、规则还是专题来划分，不要把所有资料塞进一个集合。',
    '只激活当前真正需要被 AI 消费的集合，避免召回噪音过大。',
    '把“结构边界”问题先在集合层解决，再去做具体文档导入。',
  ],
  detailSections: [
    {
      id: 'rag-collections-structure',
      title: '一、为什么先管集合',
      items: [
        '集合是知识的边界，不是简单的文件夹。',
        '集合划分错误时，后续导入和查询都会持续变差。',
        '先拆结构，再导资料，效率更高。',
      ],
    },
    {
      id: 'rag-collections-active',
      title: '二、激活状态的意义',
      items: [
        '激活集合表示它允许进入 AI 检索路径。',
        '不激活不等于删除，而是暂时不参与召回。',
        'agent_memory 这类特殊集合有自己的固定边界，不按普通集合处理。',
      ],
    },
    {
      id: 'rag-collections-flow',
      title: '三、推荐顺序',
      ordered: true,
      items: [
        '先定义集合主题和命名。',
        '创建集合并补充描述。',
        '按当前使用场景决定是否激活。',
        '再进入导入和查询阶段验证效果。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const zhRagIngestQueryEntry = {
  id: 'feature-rag-ingest-query',
  icon: 'fas fa-file-import',
  title: '知识库管理 / 导入与查询',
  route: '/rag-management -> 文档导入 / 查询',
  summary:
    '这部分处理资料真正进入检索链路后的两个核心动作：导入文档，以及查询验证召回是否符合预期。',
  capabilities: [
    '覆盖文档导入、详情查看、集合查询和结果验证。',
    '适合检查文档是否真的进入集合，而不是只停留在上传动作完成。',
    '强调“导入成功”不等于“AI 一定会正确消费”，必须继续做查询验证。',
  ],
  operations: [
    '先把资料导入目标集合，再用查询验证召回质量。',
    '如果查询结果偏差很大，先回头看集合边界和文档内容，而不是直接怪模型。',
    '查询通过后，再回到 AI 助手验证这些知识是否真正影响答案。',
  ],
  detailSections: [
    {
      id: 'rag-ingest-steps',
      title: '一、导入阶段',
      items: [
        '选择目标集合并导入文档。',
        '检查文档数量、详情和统计是否更新。',
        '确认导入的内容和集合主题一致。',
      ],
    },
    {
      id: 'rag-query-steps',
      title: '二、查询阶段',
      items: [
        '直接对集合发查询，观察召回结果是否命中预期文档。',
        '不要跳过这一步，否则你无法知道问题出在资料、结构还是后续 AI 使用。',
        '先验证集合本身，再验证 AI 助手是否消费到了这些结果。',
      ],
    },
    {
      id: 'rag-ingest-query-boundary',
      title: '三、判断边界',
      items: [
        '导入成功只说明资料进了系统。',
        '查询正确才说明检索链路大致可用。',
        'AI 回复变好，才说明端到端效果真正成立。',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enPluginLifecycleEntry = {
  id: 'feature-plugin-lifecycle',
  icon: 'fas fa-plug-circle-plus',
  title: 'Plugin Management / Lifecycle',
  route: '/plugins -> Create / Upload / AI Generate / Toggle / Test',
  summary:
    'This entry focuses on the plugin lifecycle from creation to enablement. It separates creation, upload, AI generation, testing, and activation from the plugin overview.',
  capabilities: [
    'Cover create, upload, and AI-generated plugin entry paths.',
    'Cover regular testing, advanced testing, code review, and enablement.',
    'Emphasize that enablement is not enough; you still need to verify the plugin inside the target workflow.',
  ],
  operations: [
    'Choose the source path first: create, upload, or AI generation.',
    'Finish the metadata and code, then test before enabling.',
    'Return to the target operational page and confirm the plugin is actually participating.',
  ],
  detailSections: [
    {
      id: 'plugin-lifecycle-entry',
      title: '1. Choosing the Entry Path',
      items: [
        'Create for hand-built logic from a blank starting point.',
        'Upload when the plugin package already exists.',
        'Use AI generation for a draft that still requires human review.',
      ],
    },
    {
      id: 'plugin-lifecycle-validate',
      title: '2. Test Before Enablement',
      items: [
        'Regular testing gives a quick runtime check.',
        'Advanced testing is better for realistic structured inputs.',
        'Code review and validation output are especially important for AI-generated or uploaded plugins.',
      ],
    },
    {
      id: 'plugin-lifecycle-finish',
      title: '3. Closing the Loop',
      ordered: true,
      items: [
        'Create or import the plugin.',
        'Review configuration, plugin type, category, and core logic.',
        'Run tests and fix obvious issues.',
        'Enable the plugin.',
        'Validate it from the operational page where it is supposed to run.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enPluginStoreReviewEntry = {
  id: 'feature-plugin-store-review',
  icon: 'fas fa-store',
  title: 'Plugin Management / Store and Review',
  route: '/plugins -> Plugin Store / Plugin Review',
  summary:
    'This area handles source governance and release governance: installing and updating plugins from the store, then approving or rejecting reviewed plugins before runtime use.',
  capabilities: [
    'The store supports list/card views, search, category filtering, install, update, and detail review.',
    'The review tab supports status filtering, batch approval/rejection, and detailed inspection.',
    'It separates obtaining a plugin from allowing it into the runtime.',
  ],
  operations: [
    'Use the store when the plugin comes from an external source.',
    'Use review when the plugin must be approved before runtime use.',
    'Even after approval, continue with testing and runtime verification.',
  ],
  detailSections: [
    {
      id: 'plugin-store-what',
      title: '1. What the Store Does',
      items: [
        'Load store entries and filter them by category.',
        'Inspect version, author, tags, local version, and update state.',
        'Install new plugins or update existing ones.',
      ],
    },
    {
      id: 'plugin-review-what',
      title: '2. What Review Does',
      items: [
        'Inspect pending plugins through status filters and paginated results.',
        'Open details and review code plus metadata.',
        'Approve or reject individual plugins or batches.',
      ],
    },
    {
      id: 'plugin-store-review-boundary',
      title: '3. Boundary',
      items: [
        'The store solves source and installation.',
        'Review solves runtime admission.',
        'Testing solves whether the plugin actually works in the intended scenario.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enRagCollectionsEntry = {
  id: 'feature-rag-collections',
  icon: 'fas fa-layer-group',
  title: 'RAG Management / Collections',
  route: '/rag-management -> Collections',
  summary:
    'Collection management defines the structural boundary of long-lived knowledge. It controls how knowledge is grouped, which collections are active, and which remain stored but inactive.',
  capabilities: [
    'Cover collection creation, editing, deletion, search, filtering, and stats.',
    'Support activation and deactivation to control what enters retrieval.',
    'Include the special handling boundary for collections such as agent_memory.',
  ],
  operations: [
    'Decide whether the structure should be by project, SOP, rule set, or topic before importing content.',
    'Activate only the collections that should participate in retrieval right now.',
    'Solve structure first, then move to ingestion.',
  ],
  detailSections: [
    {
      id: 'rag-collections-structure',
      title: '1. Why Collections Come First',
      items: [
        'Collections are retrieval boundaries, not only folders.',
        'Poor collection structure degrades ingestion and query quality later.',
        'Define the structure before loading content.',
      ],
    },
    {
      id: 'rag-collections-active',
      title: '2. Meaning of Activation',
      items: [
        'Active collections participate in retrieval.',
        'Inactive collections are stored but excluded from recall.',
        'Special collections such as agent_memory have their own boundary rules.',
      ],
    },
    {
      id: 'rag-collections-flow',
      title: '3. Recommended Sequence',
      ordered: true,
      items: [
        'Define names and themes.',
        'Create collections and add descriptions.',
        'Decide activation state for the current use case.',
        'Move into ingestion and query validation.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry

export const enRagIngestQueryEntry = {
  id: 'feature-rag-ingest-query',
  icon: 'fas fa-file-import',
  title: 'RAG Management / Ingest and Query',
  route: '/rag-management -> Ingest / Query',
  summary:
    'This entry covers the two critical post-structure actions: ingesting documents into collections and querying those collections to verify recall quality.',
  capabilities: [
    'Cover document ingestion, detail review, collection querying, and result validation.',
    'Help confirm that content was truly ingested instead of only uploaded.',
    'Emphasize that successful ingestion does not guarantee useful AI consumption; query validation is still required.',
  ],
  operations: [
    'Ingest content into the target collection, then query it directly.',
    'If query quality is poor, inspect collection structure and content first instead of blaming the model immediately.',
    'After query quality looks right, verify the same material is actually used by AI Assistant.',
  ],
  detailSections: [
    {
      id: 'rag-ingest-steps',
      title: '1. Ingestion Stage',
      items: [
        'Choose the target collection and ingest the documents.',
        'Check whether document counts, details, and stats changed.',
        'Confirm the imported material actually matches the collection theme.',
      ],
    },
    {
      id: 'rag-query-steps',
      title: '2. Query Stage',
      items: [
        'Query the collection directly and inspect whether expected material is recalled.',
        'Do not skip this step, or you will not know whether problems come from data, structure, or downstream AI usage.',
        'Validate the collection first, then validate AI Assistant behavior.',
      ],
    },
    {
      id: 'rag-ingest-query-boundary',
      title: '3. Boundary',
      items: [
        'Successful ingestion only proves the content entered the system.',
        'Correct query results prove the retrieval path is roughly working.',
        'Improved AI output proves the end-to-end loop is actually working.',
      ],
    },
  ],
} satisfies HelpCenterFeatureEntry
