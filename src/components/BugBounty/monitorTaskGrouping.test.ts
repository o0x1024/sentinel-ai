import { describe, expect, it } from 'vitest'

import { buildMonitorTaskDisplayGroups, getMonitorTaskChildren } from './monitorTaskGrouping'

const createTask = (overrides: Record<string, any>) => ({
  id: 'task-default',
  program_id: 'program-default',
  name: 'Default Task',
  interval_secs: 3600,
  enabled: true,
  config: {},
  next_run_at: null,
  last_run_at: null,
  run_count: 0,
  events_detected: 0,
  created_at: '2026-05-20T00:00:00Z',
  ...overrides,
})

describe('monitorTaskGrouping', () => {
  it('aggregates batch-created monitor tasks into one display group', () => {
    const groupedTasks = buildMonitorTaskDisplayGroups(
      [
        createTask({
          id: 'task-1',
          program_id: 'program-1',
          name: 'Daily Monitor - Alpha',
          group_id: 'group-1',
          group_name: 'Daily Monitor',
          enabled: false,
          run_count: 2,
          events_detected: 3,
          next_run_at: '2026-05-21T08:00:00Z',
          last_run_at: '2026-05-20T08:00:00Z',
          created_at: '2026-05-19T08:00:00Z',
        }),
        createTask({
          id: 'task-2',
          program_id: 'program-2',
          name: 'Daily Monitor - Beta',
          group_id: 'group-1',
          group_name: 'Daily Monitor',
          enabled: true,
          run_count: 5,
          events_detected: 7,
          next_run_at: '2026-05-21T07:00:00Z',
          last_run_at: '2026-05-20T09:00:00Z',
          created_at: '2026-05-19T09:00:00Z',
        }),
      ],
      [
        { id: 'program-1', name: 'Alpha' },
        { id: 'program-2', name: 'Beta' },
      ],
    )

    expect(groupedTasks).toHaveLength(1)
    expect(groupedTasks[0]).toMatchObject({
      id: 'group:group-1',
      name: 'Daily Monitor',
      program_ids: ['program-1', 'program-2'],
      program_names: ['Alpha', 'Beta'],
      enabled: true,
      run_count: 7,
      events_detected: 10,
      next_run_at: '2026-05-21T07:00:00Z',
      last_run_at: '2026-05-20T09:00:00Z',
      created_at: '2026-05-19T08:00:00Z',
      __is_group: true,
    })
    expect(getMonitorTaskChildren(groupedTasks[0]).map(task => task.id)).toEqual([
      'task-1',
      'task-2',
    ])
  })

  it('keeps ungrouped monitor tasks as normal display rows', () => {
    const task = createTask({
      id: 'task-single',
      program_id: 'program-1',
      name: 'Single Project Monitor',
    })

    const groupedTasks = buildMonitorTaskDisplayGroups(
      [task],
      [{ id: 'program-1', name: 'Alpha' }],
    )

    expect(groupedTasks).toEqual([task])
    expect(getMonitorTaskChildren(groupedTasks[0])).toEqual([task])
  })

  it('keeps a lone visible grouped task as a normal row', () => {
    const task = createTask({
      id: 'task-visible',
      program_id: 'program-1',
      name: 'Daily Monitor - Alpha',
      group_id: 'group-1',
      group_name: 'Daily Monitor',
    })

    const groupedTasks = buildMonitorTaskDisplayGroups(
      [task],
      [{ id: 'program-1', name: 'Alpha' }],
    )

    expect(groupedTasks).toEqual([task])
    expect(getMonitorTaskChildren(groupedTasks[0])).toEqual([task])
  })
})
