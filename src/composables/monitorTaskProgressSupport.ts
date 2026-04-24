export interface MonitorTaskProgressState {
  status?: string | null
  progress?: number | null
  completed_steps?: number | null
  total_steps?: number | null
  current_plugin?: string | null
  current_plugin_index?: number | null
  target_count?: number | null
  imported_assets?: number | null
  message?: string | null
  execution_mode?: string | null
  started_at?: string | null
  updated_at?: string | null
  scan_completed_targets?: number | null
  scan_total_targets?: number | null
  scan_completed_units?: number | null
  scan_total_units?: number | null
  current_target?: string | null
  plugin_completed_units?: number | null
  plugin_total_units?: number | null
  plugin_phase?: string | null
  plugin_phase_label?: string | null
  indeterminate?: boolean
  [key: string]: unknown
}

const toNonNegativeNumber = (value: unknown) => {
  const normalized = Number(value)
  if (!Number.isFinite(normalized)) {
    return 0
  }
  return Math.max(0, normalized)
}

const clampProgress = (value: unknown, max = 100) =>
  Math.min(max, Math.max(0, Math.round(toNonNegativeNumber(value))))

const resolveDetailedFraction = (progress: MonitorTaskProgressState | null | undefined) => {
  const scanCompletedUnits = toNonNegativeNumber(progress?.scan_completed_units)
  const scanTotalUnits = toNonNegativeNumber(progress?.scan_total_units)
  if (scanTotalUnits > 0) {
    return Math.min(scanCompletedUnits, scanTotalUnits) / scanTotalUnits
  }

  const scanCompletedTargets = toNonNegativeNumber(progress?.scan_completed_targets)
  const scanTotalTargets = toNonNegativeNumber(progress?.scan_total_targets)
  if (scanTotalTargets > 0) {
    return Math.min(scanCompletedTargets, scanTotalTargets) / scanTotalTargets
  }

  const pluginCompletedUnits = toNonNegativeNumber(progress?.plugin_completed_units)
  const pluginTotalUnits = toNonNegativeNumber(progress?.plugin_total_units)
  if (pluginTotalUnits > 0) {
    return Math.min(pluginCompletedUnits, pluginTotalUnits) / pluginTotalUnits
  }

  return null
}

const hasIncompleteScanProgress = (progress: MonitorTaskProgressState | null | undefined) => {
  const totalTargets = toNonNegativeNumber(progress?.scan_total_targets)
  const completedTargets = toNonNegativeNumber(progress?.scan_completed_targets)
  return totalTargets > 0 && completedTargets < totalTargets
}

const hasIncompletePluginProgress = (progress: MonitorTaskProgressState | null | undefined) => {
  const totalUnits = toNonNegativeNumber(progress?.plugin_total_units)
  const completedUnits = toNonNegativeNumber(progress?.plugin_completed_units)
  return totalUnits > 0 && completedUnits < totalUnits
}

export const deriveMonitorTaskProgressValue = (
  progress: MonitorTaskProgressState | null | undefined,
) => {
  if (!progress) return 0

  const detailedFraction = resolveDetailedFraction(progress)
  if (detailedFraction == null || progress.status !== 'running') {
    return clampProgress(progress.progress)
  }

  const totalSteps = clampProgress(progress.total_steps, Number.MAX_SAFE_INTEGER)
  const completedSteps = Math.min(
    clampProgress(progress.completed_steps, Number.MAX_SAFE_INTEGER),
    totalSteps,
  )

  if (totalSteps <= 0) {
    return clampProgress(detailedFraction * 100, 99)
  }

  return clampProgress(((completedSteps + detailedFraction) / totalSteps) * 100, 99)
}

export const mergeMonitorTaskProgress = (
  previous: MonitorTaskProgressState | null | undefined,
  payload: MonitorTaskProgressState,
) => {
  const previousState = previous || {}
  const sameRunningPlugin = payload.status === 'running'
    && previousState.status === 'running'
    && payload.current_plugin
    && previousState.current_plugin
    && payload.current_plugin === previousState.current_plugin

  const mergedState: MonitorTaskProgressState = {
    ...previousState,
    ...payload,
  }

  if (sameRunningPlugin && hasIncompleteScanProgress(previousState)) {
    if (payload.scan_completed_targets == null) {
      mergedState.scan_completed_targets = previousState.scan_completed_targets
    }
    if (payload.scan_total_targets == null) {
      mergedState.scan_total_targets = previousState.scan_total_targets
    }
    if (payload.scan_completed_units == null) {
      mergedState.scan_completed_units = previousState.scan_completed_units
    }
    if (payload.scan_total_units == null) {
      mergedState.scan_total_units = previousState.scan_total_units
    }
    if (payload.current_target == null) {
      mergedState.current_target = previousState.current_target
    }
  }

  if (sameRunningPlugin && hasIncompletePluginProgress(previousState)) {
    if (payload.plugin_completed_units == null) {
      mergedState.plugin_completed_units = previousState.plugin_completed_units
    }
    if (payload.plugin_total_units == null) {
      mergedState.plugin_total_units = previousState.plugin_total_units
    }
    if (payload.plugin_phase == null) {
      mergedState.plugin_phase = previousState.plugin_phase
    }
    if (payload.plugin_phase_label == null) {
      mergedState.plugin_phase_label = previousState.plugin_phase_label
    }
    if (payload.current_target == null) {
      mergedState.current_target = previousState.current_target
    }
  }

  if (mergedState.status === 'running' && resolveDetailedFraction(mergedState) != null) {
    mergedState.indeterminate = false
    mergedState.progress = deriveMonitorTaskProgressValue(mergedState)
  }

  return mergedState
}
