import { atom } from 'jotai'

import { TargetState } from '@gitmono/types/generated'

export type BuildStatus = 'Pending' | 'Completed' | 'Failed' | 'Building' | 'Interrupted'

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
  NotFound = 'NotFound'
}

export const logsAtom = atom<Record<string, string>>({})
export const statusAtom = atom<Record<string, Status>>({})
export const loadingAtom = atom(true)
export const statusMapAtom = atom<Map<string, BuildDTO>>(new Map())
export const tabAtom = atom<'conversation' | 'check' | 'filechange'>('conversation')
