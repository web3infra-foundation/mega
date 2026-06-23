import { TaskInfoDTO } from './store'

/**
 * Stable signature so effects only re-run when build ids/statuses or target states
 * change. Target states must be included: a build can stay `Building` while
 * targets move from `Uninitialized` (queued) to `Building` (worker picked up).
 */
export function getTasksSignature(tasks: TaskInfoDTO[]): string {
  return tasks
    .map((t) => {
      const builds = t.build_list?.map((b) => `${b.id}:${b.status}`).join(',') ?? ''
      const targets = t.targets?.map((tg) => tg.state).join(',') ?? ''

      return `${t.task_id}:${builds}:${targets}`
    })
    .join('|')
}
