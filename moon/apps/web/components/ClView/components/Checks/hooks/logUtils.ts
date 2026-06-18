import { GetBuildsLogsV2Data } from '@gitmono/types/generated'

import { BuildDTO, BuildStatus, TaskInfoDTO } from '../cpns/store'

export type LogStatus = 'idle' | 'loading' | 'success' | 'empty' | 'error'

export const TERMINAL_BUILD_STATUSES = new Set<BuildStatus>(['Completed', 'Failed', 'Interrupted'])

export const MAX_MOUNTED_LOG_PANELS = 6

export function parseBuildLogResponse(res: GetBuildsLogsV2Data | null | undefined): {
  status: LogStatus
  text: string
} {
  if (!res || !res.data) {
    return { status: 'empty', text: '' }
  }

  if (Array.isArray(res.data) && res.data.length === 0) {
    return { status: 'empty', text: '' }
  }

  if (res.len === 0) {
    return { status: 'empty', text: '' }
  }

  const text = Array.isArray(res.data) ? res.data.join('\n') : String(res.data || '')

  if (!text) {
    return { status: 'empty', text: '' }
  }

  return { status: 'success', text }
}

export function findBuildInTasks(tasks: TaskInfoDTO[], buildId: string): BuildDTO | undefined {
  for (const task of tasks) {
    const build = task.build_list?.find((b) => b.id === buildId)

    if (build) return build
  }

  return undefined
}

/** Adjacent build ids in chronological order within the same task. */
export function getAdjacentBuildIds(tasks: TaskInfoDTO[], buildId: string): string[] {
  for (const task of tasks) {
    const list = [...(task.build_list ?? [])].sort(
      (a, b) => new Date(a.start_at).getTime() - new Date(b.start_at).getTime()
    )
    const index = list.findIndex((b) => b.id === buildId)

    if (index === -1) continue

    const adjacent: string[] = []

    if (index > 0) adjacent.push(list[index - 1].id)
    if (index < list.length - 1) adjacent.push(list[index + 1].id)

    return adjacent
  }

  return []
}
