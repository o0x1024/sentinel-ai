export interface TeamProfilePresetOption {
  id: string
  label: string
  description: string
}

export const TEAM_ORCHESTRATION_PRESET_METAS: TeamProfilePresetOption[] = [
  {
    id: 'product_delivery_chain',
    label: '需求到交付',
    description: '产品 -> 架构 -> 研发/测试并行 -> 发布决策，适合 idea 到落地全链路。',
  },
  {
    id: 'incident_response_flow',
    label: '故障处置链路',
    description: '故障接管 -> 并行根因分析 -> 处置方案 -> 验证复盘，适合线上异常场景。',
  },
]

export const TEAM_RECOVERY_PRESETS: TeamProfilePresetOption[] = [
  {
    id: 'conservative',
    label: '保守',
    description: '低重试、长退避、较长人工等待窗口，优先稳定性与可回滚性。',
  },
  {
    id: 'balanced',
    label: '平衡',
    description: '中等重试与等待窗口，在质量、风险和进度间折中。',
  },
  {
    id: 'aggressive',
    label: '激进',
    description: '高重试、短退避、短等待窗口，优先推进速度与产出。',
  },
]
