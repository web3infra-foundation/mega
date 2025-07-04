import { CookieValueTypes } from 'cookies-next'
import { atomFamily } from 'jotai/utils'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

export type IssueIndexFilterType = 'open' | 'closed' | 'Merged' | 'draft'

export const filterAtom = atomFamily(
  ({ scope, part }: { scope: CookieValueTypes; part: string }) =>
    atomWithWebStorage<IssueIndexFilterType>(`${scope}:${part}-index-filter`, 'open'),
  (a, b) => a.scope === b.scope && a.part === b.part
)

export interface IssueSortType {
  [key: string]: string | string[]
}

export const sortAtom = atomFamily(
  ({ scope, filter }: { scope: CookieValueTypes; filter: string }) =>
    atomWithWebStorage<IssueSortType>(`${scope}:issue-index-sort:${filter}`, { Author: '', Assignees: [] }),
  (a, b) => a.scope === b.scope && a.filter === b.filter
)

export const darkModeAtom = atomWithWebStorage<boolean>('darkMode', false)

export const currentPage = atomWithWebStorage<number>('currentPage', 1)
