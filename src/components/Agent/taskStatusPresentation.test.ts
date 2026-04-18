import { describe, expect, it } from 'vitest'

import { getAgentTaskIndicatorClass } from './taskPresentation'
import { teamTaskStatusBadgeClass } from './teamWorkspacePresentation'
import {
  getAgentTaskStatusTone,
  getTaskToneBadgeClass,
  getTaskToneIndicatorClass,
  getTeamTaskStatusTone,
} from './taskStatusPresentation'

describe('taskStatusPresentation', () => {
  it('maps agent task statuses through shared tones', () => {
    expect(getAgentTaskStatusTone('in_progress')).toBe('active')
    expect(getTaskToneIndicatorClass(getAgentTaskStatusTone('in_progress'))).toBe('text-primary')
    expect(getAgentTaskIndicatorClass('in_progress')).toBe('text-primary')

    expect(getAgentTaskStatusTone('cancelled')).toBe('muted')
    expect(getAgentTaskIndicatorClass('cancelled')).toBe('text-base-content/50')
  })

  it('maps team task statuses through shared tones', () => {
    expect(getTeamTaskStatusTone('running')).toBe('active')
    expect(getTaskToneBadgeClass(getTeamTaskStatusTone('running'))).toBe('badge-info')
    expect(teamTaskStatusBadgeClass('running')).toBe('badge-info')

    expect(getTeamTaskStatusTone('ready_for_claim')).toBe('accent')
    expect(teamTaskStatusBadgeClass('ready_for_claim')).toBe('badge-secondary')
  })
})
