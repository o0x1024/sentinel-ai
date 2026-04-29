import type { TeamV4SolverAssignment } from '@/types/teamRuntime'

export const runTeamV4AssignmentsWithDependencies = async <T>(
  assignments: TeamV4SolverAssignment[],
  maxSolvers: number,
  worker: (assignment: TeamV4SolverAssignment, index: number) => Promise<T>,
) => {
  const idToIndex = new Map(assignments.map((assignment, index) => [assignment.task.id, index]))
  assignments.forEach((assignment) => {
    const dependencies = Array.isArray(assignment.task.depends_on) ? assignment.task.depends_on : []
    dependencies.forEach((dependency) => {
      if (typeof dependency !== 'string' || !idToIndex.has(dependency)) {
        throw new Error(`Team v4 task ${assignment.task.task_key} has unknown dependency: ${String(dependency)}.`)
      }
    })
  })

  const results = new Array<T>(assignments.length)
  const completed = new Set<string>()
  const running = new Set<string>()
  const pending = new Set(assignments.map((assignment) => assignment.task.id))
  const concurrency = Math.max(1, Math.floor(maxSolvers))
  let rejected = false

  return new Promise<T[]>((resolve, reject) => {
    const launchReady = () => {
      if (rejected) return
      if (pending.size === 0 && running.size === 0) {
        resolve(results)
        return
      }

      let launched = false
      for (const assignment of assignments) {
        if (running.size >= concurrency) break
        if (!pending.has(assignment.task.id)) continue
        const dependencies = Array.isArray(assignment.task.depends_on) ? assignment.task.depends_on : []
        const ready = dependencies.every((dependency) => completed.has(String(dependency)))
        if (!ready) continue
        launched = true
        pending.delete(assignment.task.id)
        running.add(assignment.task.id)
        const index = idToIndex.get(assignment.task.id)!
        void worker(assignment, index)
          .then((result) => {
            results[index] = result
            running.delete(assignment.task.id)
            completed.add(assignment.task.id)
            launchReady()
          })
          .catch((error) => {
            rejected = true
            reject(error)
          })
      }

      if (!launched && running.size === 0 && pending.size > 0) {
        rejected = true
        reject(new Error('Team v4 task graph has unsatisfied dependencies.'))
      }
    }

    launchReady()
  })
}
