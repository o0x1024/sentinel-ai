1. 核心规划 (Planning)
所有的设计应围绕 “Event-Driven Actor Model” (事件驱动的 Actor 模型) 展开，将“思考”、“记忆”和“行动”完全解耦。

新架构分层：
大脑层 (The Brain - Agents)：
Planner (规划者)：只负责决策。它看一眼当前的“世界地图”，决定下一步去哪里（BFS/DFS 策略在此实现）。
Navigator (从者/导航员)：只负责“动”。给定一个目标 URL 或路径，它负责操作浏览器到达那里（处理 404、重定向）。
Analyst (分析师)：只负责“看”。它负责 VLM 视觉分析、页面解析、提取 API 和表单。
Operator (操作员)：只负责“交互”。专门处理复杂的交互，如填表单、处理弹窗、Solve Captcha。
记忆层 (The Memory - World Model)：
Graph State (图状态)：这是核心。不再是简单的 visited_urls 列表，而是一个有向图。
Node: PageState (URL + Hash)。
Edge: Action (点击 Button A -> 导致跳转到 Node B)。
Blackboard (黑板)：共享数据区，存放 Cookie、Auth Token、全局配置。
身体层 (The Body - Driver)：
BrowserDriver: 一个极简的、无状态的执行器。它不知道“任务”是什么，只知道 click(x, y)、goto(url)、screenshot()。
2. 实现路径 (How to Implement)
建议采用**“全新的目录结构并行开发，逐步迁移”**的策略。

第一阶段：基础设施 (Infrastructure)
新建 src-tauri/src/engines/vision_explorer_v2/ 目录。
Core: 定义 Engine (事件总线) 和 
Agent
 trait。
State: 实现 ExplorationGraph（基于 petgraph 或简单邻接表），用来存储发现的网站结构。
第二阶段：拆解巨石 (Breaking the Monolith)
Driver: 重写 BrowserTools。目前的 BrowserTools 混杂了 MCP 调用和业务逻辑。新版应仅仅是 CDP/Playwright 的 Rust 包装。
Agents:
将 
WorkerAgent
 中的 
partition_elements
 等视觉过滤逻辑剥离给 Analyst。
将导航和回退逻辑 (go_back, 
is_in_scope
) 剥离给 Navigator。
将登录检测 (detect_login_page) 和 TakeoverManager 封装进 AuthAgent。
第三阶段：组装与替换 (Assembly)
在 core/engine.rs 中实现 tick() 循环：
从队列取 Event。
分发给订阅该 Event 的 Agent。
Agent 返回 Action。
Driver 执行 Action。
更新 World Model。
最后修改对外接口，将流量切换到 v2 引擎。

 原因 (Why & Reasons)
解耦职责 (Decoupling)：
目前的 
WorkerAgent
 既要管走路（导航），又要管看路（VLM），还要管修路（登录/报错）。重构后，增加一种新的“验证码识别能力”只需要加一个 Agent，不需要修改几千行的 Worker 文件。
状态一致性 (State Consistency)：
目前 
GlobalExplorerState
 和 StateManager 功能重叠，且主要存的是“访问记录”。
原因：网站本质是一个图。用图结构存储（Graph State）能天然解决“我是怎么来到这一页的”（路径回溯）和“这里的回退按钮会去哪”的问题，极其利于 LLM 理解上下文。
可测试性 (Testability)：
目前的架构很难写单元测试，因为都依赖真实的浏览器环境。
原因：拆分后，Planner 可以针对模拟的图数据进行路径规划测试，完全不需要启动浏览器。
容错与恢复 (Resilience)：
目前的递归/循环调用一旦中间崩了，很难恢复上下文。
原因：事件驱动架构下，每个状态都是持久化的。如果程序崩溃，重启后加载“图状态”，Planner 看一眼图就知道还没探索完哪些分支，直接继续。


 高级策略：混合模式 (Hybrid Strategy)
最强大的系统不是二选一，而是协同。在 V2 架构中，PlannerAgent 可以动态调度：

Fast-Pass (快速通道)：默认使用 StructuralAnalyst (LLM) 快速扫过 90% 的普通页面（链接、文本页）。
Deep-Dive (深潜模式)：一旦 LLM 报告“我找不到按钮”或者“页面结构非常奇怪（可能是 Canvas）”，Planner 立即切换 VisualAnalyst (VLM) 介入，进行截图分析。
Cross-Check (交叉验证)：在关键操作（如支付、删除数据）前，同时调用两者，确保 DOM 里的“删除”按钮在视觉上确实是红色的且没有被弹窗遮挡。

在新的 V2 架构规划中，多 Agent 探索将从目前的**“切分地盘式并行” (Static Partitioning)** 进化为更灵活的**“蜂群式协作” (Swarm Intelligence)**。

目前的架构更像是“包产到户”：Manager 把网站切成三块，分给三个 Worker，大家各干各的，老死不相往来。如果 Worker A 很快干完了，Worker B 卡住了，A 也没法帮忙。

在 V2 架构中，我准备这样处理多 Agent：

1. 核心转变：从“任务分配”到“动态调度”
不再是一开始就硬性划分 Scope，而是维护一个全局的**“探索前沿” (Exploration Frontier)** 队列。

Global Planner (原 Manager 的进化)：它是“蜂巢大脑”。它看着全局的 ExplorationGraph，实时计算哪些节点还需要探索，将任务放入队列。
Explorer Unit (原 Worker 的进化)：它是“工蜂”。它不再绑定死一个 URL Scope，而是是一个无状态的执行单元。它从队列领任务 -> 执行 -> 汇报结果 -> 领下一个任务。
2. 角色细分与协作 (Specialization)
在 V2 中，不同的 Agent 可以有不同的“配置”，形成互补的团队：

Scout Agent (侦察兵)：
配置：LLM 模式 (Fast/Cheap)。
任务：快速点击链接，扩充图的节点，不深究细节。
行为：遇到复杂表单或登录页，它不处理，直接标记为“高价值/困难节点”，扔回图里。
Specialist Agent (特种兵)：
配置：VLM 模式 (Slow/Smart)。
任务：专门从图中领取被标记为“困难”的任务。
行为：负责填写复杂的多步表单、处理验证码、分析 Canvas 图表。
3. 具体实现架构 (Implementation)
在 vision_explorer_v2 中，这将体现为：

A. 共享数据层 (The Shared Truth)
所有 Agent 共享同一个内存中的图数据库 (Arc<RwLock<ExplorationGraph>>)。

状态同步：当 Agent A 发现一个新链接 /api/v2，它立即写入图。Agent B 如果正准备去 /api/v2，会发现由于 A 已经去过了，它就不用去了（实时去重）。


Step Id: 28

新架构设计的核心目标就是“通用性”， Theoretically（理论上）它可以覆盖你提到的所有类型，因为它模仿的是人类的操作模式——“看 -> 思考 -> 操作 -> 观测变化”。人类能浏览的网站，这套架构理应都能探索。

但是，“能覆盖” 和 “完美支持” 是两回事。不同类型的网站对架构中不同组件的压力点不同。以下是详细覆盖分析及潜在局限：

1. 覆盖能力分析
A. 传统多页网站 (Server-Side Rendered / Static Sites)
特点：点击链接 -> 浏览器 URL 变更 -> 页面完全重载。
V2 应对：完美适配。
这是最简单的模式。NavigatorAgent 只要监控 URL 变化就能精准切分图的节点（Node）。
LLM 模式即可处理，速度快，成本低。
B. SPA 单页面应用 (React/Vue/Angular)
特点：点击按钮 -> URL 可能不变（或仅 Hash 变） -> DOM 局部刷新 -> 状态改变。
V2 应对：强力适配（这是 V2 最大的改进点）。
难点：如何判断“我到了新页面”？
方案：V2 的 Graph Node 定义不再只看 URL，而是引入 “State Fingerprint” (状态指纹)。
Analyst 会计算当前页面可视结构的 Hash。如果点击后 URL 没变但结构大变，系统会判定进入了新节点。
VLM 甚至可以通过截图对比（Visual Diff）来辅助判断（例如：弹出了一个模态框，URL 没变，但在图里这是一个新的 Sub-State）。
C. “只有前端” (Tools/Calculators/Games)
特点：Canvas 绘图、WebGL、交互式图表、地图应用。DOM 树可能只有 <canvas> 标签，里面全是黑盒。
V2 应对：VLM 独占模式。
LLM 模式通过 DOM 会完全瞎掉。
V2 的策略是检测到 DOM 贫瘠但截图有内容时，自动切换到 VisualAnalyst。它会把页面当成图片看，输出 click(x, y) 坐标来进行探索。
D. “重后台”系统 (Admin Panels/SaaS)
特点：复杂的表单联动、CRUD 逻辑、权限控制。
V2 应对：需要“特种兵” Agent。
这里不仅是“探索链接”，而是“状态机遍历”。例如：必须先“新建用户”，列表里才会有数据，“编辑”按钮才会出现。
V2 的 Graph 结构能记录这种依赖关系（Action A 前置于 Action B）。
2. 潜在局限与挑战 (The "Gotchas")
虽然架构上支持，但实际落地有几个物理学层面的局限：

1. 逻辑因果性 (Logical Causality/Dependency)
问题：Agent 很难天然理解业务逻辑的依赖顺序。
例子：一个电商后台，必须先配置“运费模板”，才能发布“商品”。Agent 可能一直在点“发布商品”，然后报错，它可能不知道是因为缺了前一步。
V2 局限：如果不给 Agent 预设一些“业务知识（Knowledge Base）”，它可能需要在这个 Error Loop 尝试很久才能撞对运气（或者一直在撞墙）。
2. “隐形”状态 (Hidden State)
问题：某些 SPA 的状态完全在内存里（比如 Redux/Vuex），界面上几乎没变化。
例子：点击“加入对比”，按钮颜色微变。Agent 可能认为这不算新状态，从而忽略了这个路径。
V2 局限：基于视觉和 DOM 的外部观察者永远无法获知内部程序状态。这会导致图谱的不完整。
3. 破坏性操作 (Destructive Actions)
问题：探索不等于破坏。在后台系统中，Agent 可能会点击“删除所有数据”或“群发邮件”。
V2 局限：Agent 本身没有道德观。
解决方案：这需要在 V2 中引入 "Safety Layer" (安全层)，类似于 robots.txt，我们可以定义一个 SafetyPolicy，禁止 Agent 点击包含 "Delete" / "Remove" 语义的红色按钮，或者只允许在沙箱（Sandbox）账号下运行。
4. 高度动态的数据流 (Live Data Streams)
问题：像股票大盘、即时聊天这种页面，每一毫秒像素都在变，DOM 都在变。
V2 局限：State Fingerprint 会因为时间戳、跳动的数字而不断生成新节点，导致状态爆炸（State Explosion）。需要非常聪明的 Analyst 来过滤掉“噪音”。
总结
V2 架构是目前 AI 探索领域最通用的解法。它在理论上能解决 SPA 和 Canvas 问题，比 V1 强太多。但对于复杂的业务逻辑依赖和数据安全性，这不仅仅是架构问题，更多是 Agent 的推理能力 (Reasoning) 和 安全策略 (Policy) 问题，需要在架构基础之上通过 Prompt Engineering 和规则约束来补充。