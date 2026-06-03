import { describe, expect, it } from 'vitest'

import {
  deriveMonitorTaskProgressValue,
  mergeMonitorTaskProgress,
  type MonitorTaskProgressState,
} from './monitorTaskProgressSupport'

describe('monitorTaskProgressSupport', () => {
  it('derives running progress from plugin unit progress instead of coarse phase progress', () => {
    const progress: MonitorTaskProgressState = {
      status: 'running',
      progress: 50,
      completed_steps: 0,
      total_steps: 1,
      plugin_completed_units: 14_100,
      plugin_total_units: 2_199_902,
    }

    expect(deriveMonitorTaskProgressValue(progress)).toBe(1)
  })

  it('preserves detailed plugin progress across heartbeat-only updates', () => {
    const previous: MonitorTaskProgressState = {
      status: 'running',
      progress: 1,
      completed_steps: 0,
      total_steps: 1,
      current_plugin: 'subdomain_brute',
      plugin_completed_units: 14_100,
      plugin_total_units: 2_199_902,
      current_target: 'feishu.cn',
      indeterminate: false,
    }

    const merged = mergeMonitorTaskProgress(previous, {
      status: 'running',
      progress: 50,
      completed_steps: 0,
      total_steps: 1,
      current_plugin: 'subdomain_brute',
      indeterminate: true,
      message: 'Plugin subdomain_brute is still running. Elapsed: 30s',
    })

    expect(merged.progress).toBe(1)
    expect(merged.indeterminate).toBe(false)
    expect(merged.plugin_completed_units).toBe(14_100)
    expect(merged.plugin_total_units).toBe(2_199_902)
    expect(merged.current_target).toBe('feishu.cn')
  })

  it('computes overall multi-step progress from detailed unit progress', () => {
    const merged = mergeMonitorTaskProgress(null, {
      status: 'running',
      progress: 50,
      completed_steps: 1,
      total_steps: 3,
      current_plugin: 'subdomain_brute',
      plugin_completed_units: 50,
      plugin_total_units: 100,
    })

    expect(merged.progress).toBe(50)
  })
})
