import { atom } from 'jotai'

import { MRTaskStatus } from '@/hooks/SSE/useGetMrTaskStatus'

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
export const statusMapAtom = atom<Map<string, MRTaskStatus>>(new Map())
export const tabAtom = atom<'conversation' | 'check' | 'filechange'>('conversation')
