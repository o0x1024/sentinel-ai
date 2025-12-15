export default {
  title: 'Scheduler Strategy Configuration',
  description: 'Configure AI models and strategies for different execution stages',
  currentStrategy: 'Current Strategy',
  disabled: 'Disabled',
  enableScheduler: 'Enable Scheduler',
  enableSchedulerDesc: 'Enable AI task scheduling and automatic replanning functionality',
  status: 'Status',
  enabled: 'Enabled',
  modelsConfigured: 'Models Configured',
  performance: 'Performance Level',
  estimatedCost: 'Estimated Cost',
  high: 'High',
  medium: 'Medium',
  low: 'Low',
  highCost: 'High Cost',
  mediumCost: 'Medium Cost',
  lowCost: 'Low Cost',
  required: 'Required',
  optional: 'Optional',
  selectModel: 'Select Model',
  selectProvider: 'Select Provider',
  provider: 'Provider',
  model: 'Model',
  useDefault: 'Use Default',
  maxRetries: 'Max Retries',
  timeoutSeconds: 'Timeout (seconds)',
  scenarioLabel: 'Scenario Configuration',
  stageModels: 'Stage Model Configuration',
  noModelsAvailable: 'No Models Available',
  noModelsAvailableDesc: 'Please enable AI providers and configure API keys in the AI Service Configuration section first.',
  intentAnalysisModel: 'Intent Analysis Model',
  intentAnalysisModelDesc: 'Used to analyze user input intent',
  plannerModel: 'Planner Model',
  plannerModelDesc: 'Used to generate execution plans',
  replannerModel: 'Replanner Model',
  replannerModelDesc: 'Used to dynamically adjust execution plans',
  executorModel: 'Executor Model',
  executorModelDesc: 'Used to execute specific tasks',
  evaluatorModel: 'Evaluator Model',
  evaluatorModelDesc: 'Used to evaluate execution results',
  replanningStrategy: 'Replanning Strategy Configuration',
  defaultStrategy: 'Default Replanning Strategy',
  defaultStrategyDesc: 'Strategy to use when replanning is needed',
  strategyName: 'Strategy Name',
  strategyDescription: 'Description',
  applicableScenarios: 'Applicable Scenarios',
  quickPresets: 'Quick Configuration Presets',
  applyConfig: 'Apply Configuration',
  saveConfig: 'Save Scheduler Configuration',
  configSaved: 'Scheduler configuration saved successfully',
  configSaveFailed: 'Failed to save scheduler configuration',
  highPerformance: 'High Performance configuration applied',
  balanced: 'Balanced configuration applied',
  economic: 'Economic configuration applied',
  scenarios: 'Scenario Configuration',
  strategies: {
    adaptive: 'Adaptive Strategy',
    conservative: 'Conservative Strategy',
    aggressive: 'Aggressive Strategy',
    costOptimized: 'Cost Optimized Strategy',
    adaptiveDesc: 'Automatically selects the best replanning strategy based on failure type',
    conservativeDesc: 'Conservative replanning strategy that prioritizes stability',
    aggressiveDesc: 'Aggressive replanning strategy that pursues optimal performance',
    costOptimizedDesc: 'Cost-optimized replanning strategy that balances performance and cost',
    adaptiveDetailed: {
      name: 'Adaptive Strategy',
      description: 'Automatically select the best strategy based on context',
      scenario: 'All scenarios'
    },
    scenarios: 'Scenario Configuration',
    complete: {
      name: 'Complete Replanning',
      description: 'Completely regenerate the execution plan',
      scenario: 'Severe failures'
    },
    partial: {
      name: 'Partial Replanning',
      description: 'Only adjust the failed part of the plan',
      scenario: 'Partial failures'
    },
    parameter: {
      name: 'Parameter Adjustment',
      description: 'Only adjust execution parameters',
      scenario: 'Parameter issues'
    },
    reorder: {
      name: 'Task Reordering',
      description: 'Adjust the execution order of tasks',
      scenario: 'Dependency issues'
    },
    resource: {
      name: 'Resource Reallocation',
      description: 'Reallocate computing resources',
      scenario: 'Resource shortage'
    },
    alternative: {
      name: 'Alternative Tool',
      description: 'Use alternative tools to complete tasks',
      scenario: 'Tool failures'
    }
  },
  scenarioConfig: {
    allScenarios: 'All scenarios',
    severeFail: 'Severe failures',
    partialFail: 'Partial failures',
    parameterIssue: 'Parameter issues',
    dependencyIssue: 'Dependency issues',
    resourceShortage: 'Resource shortage',
    toolFailure: 'Tool failures'
  },
  presets: {
    highPerformance: {
      title: 'High Performance',
      description: 'Use the most powerful models for complex tasks'
    },
    balanced: {
      title: 'Balanced',
      description: 'Balance between performance and cost'
    },
    economic: {
      title: 'Economic',
      description: 'Prioritize cost control'
    }
  }
}