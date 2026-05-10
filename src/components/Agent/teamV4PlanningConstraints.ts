export interface TeamV4PlannedTask {
  key: string
  title: string
  instruction: string
  acceptanceCriteria: string
  specialistId: string
  requiredTools: string[]
  dependsOnTaskKeys: string[]
  distinctFromTaskKeys: string[]
  priority: number
}

export interface TeamV4PlanningConstraintIssue {
  code:
    | 'unknown_distinct_task'
    | 'distinct_specialist_conflict'
    | 'specialist_capacity_exceeded'
  message: string
  taskKeys: string[]
  specialistId?: string
}

export const validateTeamV4PlannedTasks = (params: {
  plannedTasks: TeamV4PlannedTask[]
  availableSpecialistIds: Set<string>
  maxTasksPerSpecialist: number
}) => {
  const issues: TeamV4PlanningConstraintIssue[] = []
  const tasksByKey = new Map(params.plannedTasks.map((task) => [task.key, task]))
  const specialistTaskCounts = new Map<string, number>()
  const distinctConflictPairs = new Set<string>()

  for (const task of params.plannedTasks) {
    specialistTaskCounts.set(
      task.specialistId,
      (specialistTaskCounts.get(task.specialistId) || 0) + 1,
    )
    for (const otherKey of task.distinctFromTaskKeys) {
      const otherTask = tasksByKey.get(otherKey)
      if (!otherTask) {
        issues.push({
          code: 'unknown_distinct_task',
          message: `Task ${task.key} references unknown distinctFromTaskKeys item: ${otherKey}.`,
          taskKeys: [task.key, otherKey],
        })
        continue
      }
      if (!params.availableSpecialistIds.has(otherTask.specialistId)) {
        issues.push({
          code: 'unknown_distinct_task',
          message: `Task ${task.key} references unavailable specialist on ${otherKey}.`,
          taskKeys: [task.key, otherKey],
          specialistId: otherTask.specialistId,
        })
        continue
      }
      if (otherTask.specialistId === task.specialistId) {
        const pairKey = [task.key, otherKey].sort().join('::')
        if (distinctConflictPairs.has(pairKey)) {
          continue
        }
        distinctConflictPairs.add(pairKey)
        issues.push({
          code: 'distinct_specialist_conflict',
          message:
            `Tasks ${task.key} and ${otherKey} must use different specialists, ` +
            `but both are assigned to ${task.specialistId}.`,
          taskKeys: [task.key, otherKey],
          specialistId: task.specialistId,
        })
      }
    }
  }

  for (const [specialistId, count] of specialistTaskCounts.entries()) {
    if (count <= params.maxTasksPerSpecialist) continue
    issues.push({
      code: 'specialist_capacity_exceeded',
      message:
        `Specialist ${specialistId} was assigned ${count} tasks, exceeding ` +
        `maxTasksPerSpecialist=${params.maxTasksPerSpecialist}.`,
      taskKeys: params.plannedTasks
        .filter((task) => task.specialistId === specialistId)
        .map((task) => task.key),
      specialistId,
    })
  }

  return issues
}

export const formatTeamV4PlanningConstraintIssues = (
  issues: TeamV4PlanningConstraintIssue[],
) => issues.map((issue, index) => `${index + 1}. ${issue.message}`)
