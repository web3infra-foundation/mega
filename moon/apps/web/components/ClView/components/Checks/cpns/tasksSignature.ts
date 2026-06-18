import { TaskInfoDTO } from './store'

/** Stable signature so effects only re-run when build ids or statuses change. */
export function getTasksSignature(tasks: TaskInfoDTO[]): string {
  return tasks.map((t) => `${t.task_id}:${t.build_list?.map((b) => `${b.id}:${b.status}`).join(',') ?? ''}`).join('|')
}
