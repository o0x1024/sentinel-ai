在多架构 Agent 系统（如 LLM Compiler, Plan-and-Execute, ReWOO）中构建高效的记忆系统，是突破 LLM 上下文限制、降低幻觉、提升任务规划与执行效率的关键。

由于这三种架构的运行机制不同（**LLM Compiler** 侧重并行与DAG调度，**Plan-and-Execute** 侧重迭代修正，**ReWOO** 侧重规划与执行解耦），记忆系统的设计必须兼顾**共享性**与**架构特异性**。

以下是构建该记忆系统的分层策略与技术实现方案：

---

### 1. 记忆系统的分层架构设计

为了适配多架构，记忆系统应分为三层：**感知层（短期/工作记忆）**、**存储层（长期/情境记忆）** 和 **元认知层（程序/反思记忆）**。

#### A. 感知层：工作记忆 (Working Memory / Scratchpad)
这部分直接与 LLM 的 Context Window 交互。
*   **针对 Plan-and-Execute:** 需要维护一个动态的 `Current State`（当前状态）。包括：原始目标、已完成步骤列表、当前步骤执行结果、待执行计划。
*   **针对 ReWOO:** 需要维护一个 `Variable Store`（变量仓库）。ReWOO 的特点是规划时使用占位符（如 `<plan_1_output>`），执行时填充。工作记忆必须是一个 Key-Value 映射表，用于存储工具返回的实际结果。
*   **针对 LLM Compiler:** 需要维护一个 `Task DAG`（任务有向无环图）。记忆即是图的状态：哪些节点已完成，哪些节点的输入依赖已经就绪。

#### B. 存储层：长期记忆 (Long-term Memory)
用于跨会话、跨任务的数据检索。
*   **语义记忆 (Semantic Memory):** 基于 RAG（向量数据库），存储领域知识文档。
*   **情境记忆 (Episodic Memory):** 存储历史对话和**完整的执行轨迹 (Trajectory)**。
    *   *结构示例：* `(User_Query, Plan, Actions_Taken, Final_Result, Success_Flag)`。

#### C. 元认知层：程序记忆 (Procedural Memory)
这是提升 Agent 能力的核心。它存储的是**“如何解决问题”**的经验。
*   **Skill Library (技能库):** 存储过去成功的 Prompt 模板或由 ReWOO 生成的高质量规划蓝图。
*   **Error Logs (错误日志):** 记录导致 Plan-and-Execute 失败的路径，防止重蹈覆辙。

---

### 2. 针对特定架构的记忆增强策略

如何利用记忆系统具体提升这三种架构的能力：

#### 场景一：优化 Plan-and-Execute (P&E)
P&E 的缺点是容易陷入循环或步骤冗余。
*   **记忆策略：** **基于轨迹的自我反思 (Reflexion over Trajectories)**。
*   **实现：**
    1.  在执行每一步前，检索“相似的历史步骤”。
    2.  如果历史上某一步导致了死循环，记忆系统向 P&E Planner 发出警告（Negative Constraints）。
    3.  **提升点：** 减少试错次数，提高规划准确性。

#### 场景二：优化 ReWOO
ReWOO 最大的挑战是规划阶段无法预知工具输出，可能导致规划偏离。
*   **记忆策略：** **少样本规划注入 (Few-Shot Plan Retrieval)**。
*   **实现：**
    1.  当用户输入任务时，先在向量库中搜索“相似任务的高质量 ReWOO 蓝图”。
    2.  将检索到的蓝图作为 Few-Shot 示例放入 Prompt。
    3.  **提升点：** 即使不观察中间结果，也能依靠历史成功的规划逻辑生成极其稳健的执行流。

#### 场景三：优化 LLM Compiler
LLM Compiler 核心是并行调用。如果有些函数计算昂贵，重复调用很浪费。
*   **记忆策略：** **全局缓存与结果复用 (Global Caching)**。
*   **实现：**
    1.  建立一个工具调用的哈希表 `Hash(Tool_Name + Args) -> Output`。
    2.  在构建 DAG 之前，检查记忆中是否存在相同的调用记录。
    3.  **提升点：** 极大降低延迟和 Token 消耗，特别是对于搜索类或数据查询类任务。

---

### 3. 构建统一记忆系统的技术实现

为了让不同架构共享记忆，建议采用 **"统一总线 + 专用适配器"** 的模式。

#### 核心组件：
1.  **Vector Database (如 Milvus/Chroma):** 存储 `Query` 的 Embedding，用于检索相似的历史任务。
2.  **Graph Database (如 Neo4j, 可选):** 用于存储复杂的实体关系或任务依赖图（适合 LLM Compiler）。
3.  **Key-Value Store (如 Redis):** 用于 ReWOO 的变量映射和 LLM Compiler 的缓存。

#### 数据流转与提升流程 (The Workflow)：

1.  **预处理 (Pre-computation):**
    *   用户输入 -> **Memory Retriever** 检索相似的历史成功案例（Plan）。
    *   检索结果注入到 System Prompt 中。

2.  **规划与执行 (Orchestration):**
    *   *如果选用了 ReWOO:* Planner 生成带有占位符的计划。Executor 执行时，将结果写入 **Redis**，后续步骤从 Redis 读取。
    *   *如果选用了 Plan-and-Execute:* 每执行一步，将 `(Step, Result)` 追加到 **Scratchpad**。每完成一步，将当前状态摘要（Summarization）存入短期记忆，避免上下文溢出。

3.  **后处理与固化 (Consolidation):**
    *   任务结束 -> **Evaluator (LLM)** 评估任务是否成功。
    *   如果成功 -> 将 `(Query, Final_Plan, Tool_Trace)` 向量化存入长期记忆。
    *   如果失败 -> 生成“反思总结 (Reflection)”，存入错误库。

---

### 4. 关键代码逻辑抽象 (Python 伪代码)

```python
class AgentMemorySystem:
    def __init__(self):
        self.vector_db = VectorDB() # Chroma/Pinecone
        self.kv_store = Redis()     # For ReWOO variables & Cache
        self.chat_history = []      # Short-term

    def retrieve_experience(self, user_query):
        # 1. 语义搜索找到相似的过去任务
        similar_tasks = self.vector_db.search(user_query, top_k=3)
        # 2. 提取成功的 Plan 逻辑
        few_shot_examples = [task['plan_trace'] for task in similar_tasks if task['success']]
        return few_shot_examples

    def store_observation(self, step_id, tool_output):
        # 用于 ReWOO 填充变量，也用于 LLM Compiler 缓存
        self.kv_store.set(step_id, tool_output)
    
    def commit_trajectory(self, query, plan, execution_log, success):
        # 任务结束后，形成长期记忆
        if success:
            self.vector_db.add(
                embedding=embed(query),
                metadata={'plan': plan, 'log': execution_log}
            )
        else:
            # 生成反思并存储，供未来避坑
            reflection = self.generate_reflection(query, execution_log)
            self.vector_db.add_negative(query, reflection)

# 在 ReWOO 中的应用示例
def rewoo_agent(query):
    memory = AgentMemorySystem()
    
    # 1. 记忆增强规划
    past_plans = memory.retrieve_experience(query)
    planner_prompt = f"User: {query}\nReference Plans: {past_plans}"
    plan = llm.generate_plan(planner_prompt) 
    
    # 2. 执行与变量填充
    for step in plan:
        if step.dependency in memory.kv_store:
             real_input = memory.kv_store.get(step.dependency)
             result = tool.execute(step.tool, real_input)
             memory.store_observation(step.id, result)
```

### 5. 总结：记忆如何提升 Agent 能力

1.  **提升准确性 (Accuracy):** 通过 `Few-Shot` 机制，Agent 不再是零样本推理，而是参考了历史最佳实践（ReWOO/Plan-and-Execute 受益最大）。
2.  **提升效率 (Efficiency):** 通过 `Global Caching`，避免重复计算；通过读取成功的 DAG 结构，减少 LLM Compiler 的规划耗时。
3.  **具备成长性 (Evolvability):** 引入 `Reflexion` 机制，Agent 随着使用次数增加，错误库和成功库都在扩大，变得越来越聪明。
4.  **处理长上下文 (Long-Context):** 良好的记忆管理（摘要、遗忘机制）使得 Plan-and-Execute 能在有限的 Context Window 下处理极长的任务链。