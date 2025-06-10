import { CookieValueTypes } from 'cookies-next'
import { atomFamily } from 'jotai/utils'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

export type IssueIndexFilterType = 'open' | 'closed' | 'Merged' | 'draft'

export const filterAtom = atomFamily((scope: CookieValueTypes) =>
  atomWithWebStorage<IssueIndexFilterType>(`${scope}:issue-index-filter`, 'open')
)

type IssueIndexSortType = 'last_activity_at' | 'created_at'

export const sortAtom = atomFamily(
  ({ scope, filter }: { scope: CookieValueTypes; filter: string }) =>
    atomWithWebStorage<IssueIndexSortType>(`${scope}:notes-index-sort:${filter}`, 'last_activity_at'),
  (a, b) => a.scope === b.scope && a.filter === b.filter
)

export const darkModeAtom = atomWithWebStorage<boolean>('darkMode', false)

export const currentPage = atomWithWebStorage<number>('currentPage', 1)
