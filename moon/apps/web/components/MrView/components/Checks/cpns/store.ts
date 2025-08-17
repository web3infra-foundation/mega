import { atom } from 'jotai'

import { MRTaskStatus } from '@/hooks/SSE/useGetMrTaskStatus'

export enum Status {
  Pending = 'pending',
  Completed = 'completed',
  Failed = 'failed',
  Building = 'building',
  Interrupted = 'interrupted',
  NotFound = 'notfound'
}

export const logsAtom = atom<Record<string, string[]>>({})
export const statusAtom = atom<Record<string, Status>>({})
export const loadingAtom = atom(false)
export const statusMapAtom = atom<Map<string, MRTaskStatus>>(new Map())
