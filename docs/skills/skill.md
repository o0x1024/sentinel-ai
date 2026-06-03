第一阶段：发现与加载（启动）
Claude Code 启动时，会扫描技能：

async function getAllCommands() {
  // Load from all sources in parallel
  let [userCommands, skillsAndPlugins, pluginCommands, builtins] =
    await Promise.all([
      loadUserCommands(),      // ~/.claude/commands/
      loadSkills(),            // .claude/skills/ + plugins
      loadPluginCommands(),    // Plugin-defined commands
      getBuiltinCommands()     // Hardcoded commands
    ]);

  return [...userCommands, ...skillsAndPlugins, ...pluginCommands, ...builtins]
    .filter(cmd => cmd.isEnabled());
}

// Specific skill loading
async function loadPluginSkills(plugin) {
  // Check if plugin has skills
  if (!plugin.skillsPath) return [];

  // Two patterns supported:
  // 1. Root SKILL.md in skillsPath
  // 2. Subdirectories with SKILL.md

  const skillFiles = findSkillMdFiles(plugin.skillsPath);
  const skills = [];

  for (const file of skillFiles) {
    const content = readFile(file);
    const { frontmatter, markdown } = parseFrontmatter(content);

    skills.push({
      type: "prompt",
      name: `${plugin.name}:${getSkillName(file)}`,
      description: `${frontmatter.description} (plugin:${plugin.name})`,
      whenToUse: frontmatter.when_to_use,  // ← Note: underscores!
      allowedTools: parseTools(frontmatter['allowed-tools']),
      model: frontmatter.model === "inherit" ? undefined : frontmatter.model,
      isSkill: true,
      promptContent: markdown,
      // ... other fields
    });
  }

  return skills;
}
对于 pdf 技能，这将生成：

{
  type: "prompt",
  name: "pdf",
  description: "Extract text from PDF documents (plugin:document-tools)",
  whenToUse: "When user wants to extract or process text from PDF files",
  allowedTools: ["Bash(pdftotext:*)", "Read", "Write"],
  model: undefined,  // Uses session model
  isSkill: true,
  disableModelInvocation: false,
  promptContent: "You are a PDF processing specialist...",
  // ... other fields
}
第二阶段：第一轮 - 用户请求与技能选择
用户发送请求：“从 report.pdf 中提取文本”。Claude 收到此消息，同时其工具数组中也包含该 Skill 工具。在 Claude 决定是否调用该 pdf 技能之前，系统必须在技能工具的描述中显示可用的技能。

技能筛选与展示
并非所有已加载的技能都会显示在技能工具中。技能必须在 frontmatter 中包含 description 或 when_to_use ，否则将被过滤掉。过滤条件：

async function getSkillsForSkillTool() {
  const allCommands = await getAllCommands();

  return allCommands.filter(cmd =>
    cmd.type === "prompt" &&
    cmd.isSkill === true &&
    !cmd.disableModelInvocation &&
    (cmd.source !== "builtin" || cmd.isModeCommand === true) &&
    (cmd.hasUserSpecifiedDescription || cmd.whenToUse)  // ← Must have one!
  );
}
技能格式化
每项技能都按照 <available_skills> 部分的格式进行设置。例如，我们假设的 pdf 技能可以格式化为：
"pdf": Extract text from PDF documents - When user wants to extract or process text from PDF files

function formatSkill(skill) {
  let name = skill.name;
  let description = skill.whenToUse
    ? `${skill.description} - ${skill.whenToUse}`
    : skill.description;

  return `"${name}": ${description}`;
}
克劳德的决策过程
现在，当用户提示：“从 report.pdf 中提取文本”时，Claude 会收到包含 Skill 工具的 API 请求，读取 <available_skills> ，并进行推理（假设如此，因为我们没有看到推理过程）：

Internal reasoning:
- User wants to "extract text from report.pdf"
- This is a PDF processing task
- Looking at available skills...
- "pdf": Extract text from PDF documents - When user wants to extract or process text from PDF files
- This matches! The user wants to extract text from a PDF
- Decision: Invoke Skill tool with command="pdf"
请注意，这里没有算法匹配。没有词汇匹配，没有语义匹配，也没有搜索。这是完全基于技能描述的 LLM 推理过程。完成后，Claude 会返回一个工具使用结果：

{
  "type": "tool_use",
  "id": "toolu_123abc",
  "name": "Skill",
  "input": {
    "command": "pdf"
  }
}
第三阶段：技能工具执行
技能工具现在开始执行。这对应于序列图中的黄色“技能工具执行”框，该工具会执行验证、权限检查、文件加载和上下文修改，然后返回结果。

第一步：验证
async validateInput({ command }, context) {
  let skillName = command.trim().replace(/^\//, "");

  // Error 1: Empty
  if (!skillName) return { result: false, errorCode: 1 };

  // Error 2: Unknown skill
  const allSkills = await getAllCommands();
  if (!skillExists(skillName, allSkills)) {
    return { result: false, errorCode: 2 };
  }

  // Error 3: Can't load
  const skill = getSkill(skillName, allSkills);
  if (!skill) return { result: false, errorCode: 3 };

  // Error 4: Model invocation disabled
  if (skill.disableModelInvocation) {
    return { result: false, errorCode: 4 };
  }

  // Error 5: Not prompt-based
  if (skill.type !== "prompt") {
    return { result: false, errorCode: 5 };
  }

  return { result: true };
}
PDF 功能通过所有验证检查 ✓

步骤二：权限检查
async checkPermissions({ command }, context) {
  const skillName = command.trim().replace(/^\//, "");
  const permContext = (await context.getAppState()).toolPermissionContext;

  // Check deny rules
  for (const [pattern, rule] of getDenyRules(permContext)) {
    if (matches(skillName, pattern)) {
      return { behavior: "deny", message: "Blocked by permission rules" };
    }
  }

  // Check allow rules
  for (const [pattern, rule] of getAllowRules(permContext)) {
    if (matches(skillName, pattern)) {
      return { behavior: "allow" };
    }
  }

  // Default: ask user
  return { behavior: "ask", message: `Execute skill: ${skillName}` };
}
Assuming no rules, user is prompted: “Execute skill: pdf?”
用户同意 ✓

步骤 3：加载技能文件并生成执行上下文修改
验证和权限获得批准后，技能工具加载技能文件并准备执行上下文修改：

async *call({ command }, context) {
  const skillName = command.trim().replace(/^\//, "");
  const allSkills = await getAllCommands();
  const skill = getSkill(skillName, allSkills);

  // Load the skill prompt
  const promptContent = await skill.getPromptForCommand("", context);

  // Generate metadata tags
  const metadata = [
    `<command-message>The "${skill.userFacingName()}" skill is loading</command-message>`,
    `<command-name>${skill.userFacingName()}</command-name>`
  ].join('\n');

  // Create messages
  const messages = [
    { type: "user", content: metadata },  // Visible to user
    { type: "user", content: promptContent, isMeta: true },  // Hidden from user, visible to Claude
    // ... attachments, permissions
  ];

  // Extract configuration
  const allowedTools = skill.allowedTools || [];
  const modelOverride = skill.model;

  // Yield result with execution context modifier
  yield {
    type: "result",
    data: { success: true, commandName: skillName },
    newMessages: messages,

    // 🔑 Execution context modification function
    contextModifier(context) {
      let modified = context;

      // Inject allowed tools
      if (allowedTools.length > 0) {
        modified = {
          ...modified,
          async getAppState() {
            const state = await context.getAppState();
            return {
              ...state,
              toolPermissionContext: {
                ...state.toolPermissionContext,
                alwaysAllowRules: {
                  ...state.toolPermissionContext.alwaysAllowRules,
                  command: [
                    ...state.toolPermissionContext.alwaysAllowRules.command || [],
                    ...allowedTools  // ← Pre-approve these tools
                  ]
                }
              }
            };
          }
        };
      }

      // Override model
      if (modelOverride) {
        modified = {
          ...modified,
          options: {
            ...modified.options,
            mainLoopModel: modelOverride
          }
        };
      }

      return modified;
    }
  };
}
技能工具返回的结果包含 newMessages （元数据 + 技能提示 + 对话上下文注入权限）和 contextModifier （工具权限 + 用于执行上下文修改的模型覆盖）。至此，序列图中的黄色“技能工具执行”框已完成。

第四阶段：发送至 API（第一轮完成）
系统构建完整的消息数组以发送到 Anthropic API。这包括对话中的所有消息以及新注入的技能消息：

// Complete message array sent to API for Turn 1
{
  model: "claude-sonnet-4-5-20250929",
  messages: [
    {
      role: "user",
      content: "Extract text from report.pdf"
    },
    {
      role: "assistant",
      content: [
        {
          type: "tool_use",
          id: "toolu_123abc",
          name: "Skill",
          input: { command: "pdf" }
        }
      ]
    },
    {
      role: "user",
      content: "<command-message>The \"pdf\" skill is loading</command-message>\n<command-name>pdf</command-name>"
      // isMeta: false (default) - VISIBLE to user in UI
    },
    {
      role: "user",
      content: "You are a PDF processing specialist...\n\n## Process\n1. Validate PDF exists\n2. Run pdftotext...",
      isMeta: true  // HIDDEN from UI, sent to API
    },
    {
      role: "user",
      content: {
        type: "command_permissions",
        allowedTools: ["Bash(pdftotext:*)", "Read", "Write"],
        model: undefined
      }
    }
  ]
}
Turn 1 Completion

以上展示了我们目前为止所完成的工作。执行上下文修饰符已应用，预先批准了后续工具调用的 Bash(pdftotext:*) 、 Read 和 Write 操作。请求已发送至 Anthropic API。至此，技能执行完成。如果这是一个普通工具，那么一切就绪。然而，技能有所不同。代理技能仅注入了对话上下文和执行上下文。这意味着我们仍然需要使用所有这些注入的上下文来调用 Claude 代理，才能完成用户的请求！

第五阶段：Bash 工具执行（工具的使用需结合技能上下文！）
克劳德收到了包含注入的对话上下文的 API 响应。技能提示改变了克劳德的行为，使其具备了以下能力：

专门的 PDF 处理指令（对话上下文）
预先批准访问 Bash(pdftotext:*) 、 Read 和 Write 工具（执行上下文）
清晰的工作流程（对话背景）
Claude 处理上下文并遵循 pdf 技能的工作流程：

I'll extract text from report.pdf. Let me process the file.

[Following pdf skill's instructions]
1. Validate that report.pdf exists
2. Run pdftotext command to extract text
3. Read the output file
4. Present the extracted text to you
Claude 使用了 Bash 工具（已预先批准，无需用户提示）：

{
  "type": "tool_use",
  "id": "toolu_456def",
  "name": "Bash",
  "input": {
    "command": "pdftotext report.pdf output.txt",
    "description": "Extract text from PDF using pdftotext"
  }
}
Bash 工具成功执行并返回结果。随后，Claude 使用 Read 工具读取输出文件，并将提取的文本呈现给用户。该技能通过在对话上下文中注入指令并修改工具权限的执行上下文，成功引导 Claude 完成了专门的 PDF 提取工作流程。

结论：心智模型回顾
Claude Code 中的技能是基于提示的对话和执行上下文修饰符 ，它们通过元工具架构工作：

要点总结：

技能是 SKILL.md 文件中的提示模板 ，而不是可执行代码。
技能工具 （大写 S）是 tools 数组中的一个元工具，用于管理单个技能，而不是系统提示符中的工具。
技能通过注入指令提示（通过 isMeta: true 消息） 来修改对话上下文。
技能通过更改工具权限和模型选择来修改执行上下文。
选择是通过 LLM 推理实现的，而不是通过算法匹配实现的。
工具权限通过修改执行上下文来限定技能执行的范围。
技能每次调用都会注入两条用户消息——一条用于用户可见的元数据，一条用于发送给 API 的隐藏指令。
优雅的设计： Claude Code 将专业知识视为修改对话上下文的提示和修改执行上下文的权限 ，而不是执行代码 ，从而实现了传统函数调用难以实现的灵活性、安全性和可组合性。