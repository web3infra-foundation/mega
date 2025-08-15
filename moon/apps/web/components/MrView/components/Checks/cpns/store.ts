import { atom } from 'jotai'

export enum Status {
  Pending = 'pending',
  Success = 'success',
  Fail = 'fail'
}

export const logsAtom = atom<Record<string, string[]>>({})
export const statusAtom = atom<Record<string, Status>>({})
export const loadingAtom = atom(false)
