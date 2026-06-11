import { atom } from 'jotai'

import { StatusProjectRelativePath, TargetState } from '@gitmono/types/generated'

export type BuildStatus = 'Pending' | 'Completed' | 'Failed' | 'Building' | 'Interrupted' | 'Uninitialized'

export interface BuildDTO {
  args: string[]
  created_at: string
  end_at?: string
  exit_code?: number
  id: string
  output_file: string
  repo: string
  retry_count: number
  start_at: string
  status: BuildStatus
  target: string
  task_id: string
}

export interface TargetDTO {
  builds: BuildDTO[]
  end_at?: string
  error_summary?: string
  id: string
  start_at: string
  state: TargetState
  target_path: string
}

export interface TaskInfoDTO {
  build_list: BuildDTO[]
  changes: StatusProjectRelativePath[]
  cl_id: number
  created_at: string
  targets: TargetDTO[]
  task_id: string
  task_name: string
  template: string
}

export enum Status {
  Pending = 'Pending',
  Completed = 'Completed',
  Failed = 'Failed',
  Building = 'Building',
  Interrupted = 'Interrupted',
  Uninitialized = 'Uninitialized',
  NotFound = 'NotFound'
}

/**
 * A task is "queued" when it has been accepted but no worker has picked it up
 * yet: its target(s) are still `Uninitialized` and none is `Building`.
 */
export const isTaskQueued = (task: TaskInfoDTO): boolean => {
  const states = task.targets?.map((t) => t.state) ?? []

  if (states.length === 0) return false

  const isBuilding = states.some((s) => s === 'Building')

  return !isBuilding && states.some((s) => s === 'Uninitialized')
}

/**
 * A task has work in flight when any target is still `Pending` / `Building` /
 * `Uninitialized`, or any build event has not finished. Retry must be disabled
 * while a task is in flight to avoid duplicate concurrent builds.
 */
export const isTaskInFlight = (task: TaskInfoDTO): boolean => {
  const targetActive = task.targets?.some(
    (t) => t.state === 'Pending' || t.state === 'Building' || t.state === 'Uninitialized'
  )
  const buildActive = task.build_list?.some((b) => b.status === 'Building')

  return Boolean(targetActive || buildActive)
}

/**
 * Id of the task's most recent build (by start time). Only this build is eligible
 * for retry; superseded builds are read-only history.
 */
export const getLatestBuildId = (task: TaskInfoDTO): string | undefined => {
  let latest: BuildDTO | undefined

  task.build_list?.forEach((build) => {
    if (!latest || new Date(build.start_at).getTime() > new Date(latest.start_at).getTime()) {
      latest = build
    }
  })

  return latest?.id
}

/**
 * Collect the build ids that are still queued (waiting for a worker). These have
 * no logs yet, so callers should avoid fetching logs for them and instead show a
 * "waiting" placeholder.
 */
export const getQueuedBuildIds = (tasks: TaskInfoDTO[]): Set<string> => {
  const ids = new Set<string>()

  tasks.forEach((task) => {
    if (!isTaskQueued(task)) return

    task.build_list?.forEach((build) => {
      if (build.status === 'Building') ids.add(build.id)
    })
  })

  return ids
}

export const logsAtom = atom<Record<string, string>>({})
export const statusAtom = atom<Record<string, Status>>({})
export const loadingAtom = atom(true)
export const statusMapAtom = atom<Map<string, BuildDTO>>(new Map())
export const tabAtom = atom<'conversation' | 'check' | 'filechange'>('conversation')
