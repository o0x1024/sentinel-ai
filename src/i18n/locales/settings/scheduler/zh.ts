export default {
  title: '调度策略配置',
  description: '为不同的执行阶段配置AI模型和策略',
  currentStrategy: '当前策略',
  disabled: '已禁用',
  enableScheduler: '启用调度器',
  enableSchedulerDesc: '启用AI任务调度和自动重新规划功能',
  status: '状态',
  enabled: '已启用',
  modelsConfigured: '模型已配置',
  performance: '性能级别',
  estimatedCost: '估计成本',
  high: '高',
  medium: '中',
  low: '低',
  highCost: '高成本',
  mediumCost: '中等成本',
  lowCost: '低成本',
  required: '必需',
  optional: '可选',
  selectModel: '选择模型',
  selectProvider: '选择服务提供商',
  provider: '服务提供商',
  model: '模型',
  useDefault: '使用默认',
  maxRetries: '最大重试次数',
  timeoutSeconds: '超时时间（秒）',
  scenarioLabel: '场景配置',
  stageModels: '阶段模型配置',
  noModelsAvailable: '无可用模型',
  noModelsAvailableDesc: '请先在AI服务配置部分启用AI服务提供商并配置API密钥。',
  intentAnalysisModel: '意图分析模型',
  intentAnalysisModelDesc: '用于分析用户输入意图',
  plannerModel: '规划模型',
  plannerModelDesc: '用于生成执行计划',
  replannerModel: '重新规划模型',
  replannerModelDesc: '用于动态调整执行计划',
  executorModel: '执行模型',
  executorModelDesc: '用于执行具体任务',
  evaluatorModel: '评估模型',
  evaluatorModelDesc: '用于评估执行结果',
  replanningStrategy: '重新规划策略配置',
  defaultStrategy: '默认重新规划策略',
  defaultStrategyDesc: '需要重新规划时使用的策略',
  strategyName: '策略名称',
  strategyDescription: '描述',
  applicableScenarios: '适用场景',
  quickPresets: '快速配置预设',
  applyConfig: '应用配置',
  saveConfig: '保存调度器配置',
  configSaved: '调度器配置保存成功',
  configSaveFailed: '保存调度器配置失败',
  highPerformance: '已应用高性能配置',
  balanced: '已应用平衡配置',
  economic: '已应用经济配置',
  scenarios: '场景配置',
  strategies: {
    adaptive: '自适应策略',
    conservative: '保守策略',
    aggressive: '激进策略',
    costOptimized: '成本优化策略',
    adaptiveDesc: '根据失败类型自动选择最佳重新规划策略',
    conservativeDesc: '保守的重新规划策略，优先考虑稳定性',
    aggressiveDesc: '激进的重新规划策略，追求最佳性能',
    costOptimizedDesc: '成本优化的重新规划策略，平衡性能和成本',
    adaptiveDetailed: {
      name: '自适应策略',
      description: '根据上下文自动选择最佳策略',
      scenario: '所有场景'
    },
    scenarios: '场景配置',
    complete: {
      name: '完整重新规划',
      description: '完全重新生成执行计划',
      scenario: '严重失败'
    },
    partial: {
      name: '部分重新规划',
      description: '仅调整计划中失败的部分',
      scenario: '部分失败'
    },
    parameter: {
      name: '参数调整',
      description: '仅调整执行参数',
      scenario: '参数问题'
    },
    reorder: {
      name: '任务重新排序',
      description: '调整任务的执行顺序',
      scenario: '依赖关系问题'
    },
    resource: {
      name: '资源重新分配',
      description: '重新分配计算资源',
      scenario: '资源短缺'
    },
    alternative: {
      name: '替代工具',
      description: '使用替代工具完成任务',
      scenario: '工具失败'
    }
  },
  scenarioConfig: {
    allScenarios: '所有场景',
    severeFail: '严重失败',
    partialFail: '部分失败',
    parameterIssue: '参数问题',
    dependencyIssue: '依赖关系问题',
    resourceShortage: '资源短缺',
    toolFailure: '工具失败'
  },
  presets: {
    highPerformance: {
      title: '高性能',
      description: '为复杂任务使用最强大的模型'
    },
    balanced: {
      title: '平衡',
      description: '在性能和成本之间取得平衡'
    },
    economic: {
      title: '经济',
      description: '优先考虑成本控制'
    }
  }
}