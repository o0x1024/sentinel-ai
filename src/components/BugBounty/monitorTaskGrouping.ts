export interface MonitorTaskGroupProgram {
  id: string
  name: string
}

export interface MonitorTaskDisplayGroup {
  id: string
  name: string
  program_id: string
  group_id: string
  group_name: string
  program_ids: string[]
  program_names: string[]
  child_tasks: any[]
  interval_secs: number
  enabled: boolean
  config: any
  next_run_at: string | null
  last_run_at: string | null
  run_count: number
  events_detected: number
  created_at: string | null
  __is_group: true
}

export const isMonitorTaskGroup = (task: any): task is MonitorTaskDisplayGroup =>
  task?.__is_group === true && Array.isArray(task?.child_tasks)

const toMillis = (value?: string | null) => {
  if (!value) return null
  const millis = Date.parse(value)
  return Number.isFinite(millis) ? millis : null
}

const minDate = (values: Array<string | null | undefined>) => {
  const sorted = values
    .map(value => ({ value: value || null, millis: toMillis(value) }))
    .filter(item => item.value && item.millis != null)
    .sort((a, b) => Number(a.millis) - Number(b.millis))
  return sorted[0]?.value || null
}

const maxDate = (values: Array<string | null | undefined>) => {
  const sorted = values
    .map(value => ({ value: value || null, millis: toMillis(value) }))
    .filter(item => item.value && item.millis != null)
    .sort((a, b) => Number(b.millis) - Number(a.millis))
  return sorted[0]?.value || null
}

export const buildMonitorTaskDisplayGroups = (
  tasks: any[],
  programs: MonitorTaskGroupProgram[]
): any[] => {
  const programNameById = new Map(programs.map(program => [program.id, program.name]))
  const groups = new Map<string, any[]>()
  const singles: any[] = []

  for (const task of tasks) {
    const groupId = String(task?.group_id || '').trim()
    if (!groupId) {
      singles.push(task)
      continue
    }
    if (!groups.has(groupId)) groups.set(groupId, [])
    groups.get(groupId)?.push(task)
  }

  const grouped = Array.from(groups.values()).flatMap(groupTasks => {
    if (groupTasks.length < 2) {
      return groupTasks
    }

    const sortedTasks = [...groupTasks].sort((a, b) =>
      String(a.name || '').localeCompare(String(b.name || ''))
    )
    const first = sortedTasks[0]
    const programIds = Array.from(
      new Set(
        sortedTasks
          .map(task => task.program_id)
          .filter(Boolean)
      )
    )
    const programNames = programIds.map(id => programNameById.get(id) || id)

    return [{
      ...first,
      id: `group:${first.group_id}`,
      name: first.group_name || first.name,
      group_id: first.group_id,
      group_name: first.group_name || first.name,
      program_id: first.program_id,
      program_ids: programIds,
      program_names: programNames,
      child_tasks: sortedTasks,
      enabled: sortedTasks.some(task => task.enabled),
      next_run_at: minDate(sortedTasks.map(task => task.next_run_at)),
      last_run_at: maxDate(sortedTasks.map(task => task.last_run_at)),
      run_count: sortedTasks.reduce((sum, task) => sum + Number(task.run_count || 0), 0),
      events_detected: sortedTasks.reduce(
        (sum, task) => sum + Number(task.events_detected || 0),
        0
      ),
      created_at: minDate(sortedTasks.map(task => task.created_at)),
      __is_group: true,
    } satisfies MonitorTaskDisplayGroup]
  })

  return [...grouped, ...singles].sort((a, b) => {
    const aTime = toMillis(a.created_at) || 0
    const bTime = toMillis(b.created_at) || 0
    return bTime - aTime
  })
}

export const getMonitorTaskChildren = (task: any): any[] =>
  isMonitorTaskGroup(task) ? task.child_tasks : [task]
