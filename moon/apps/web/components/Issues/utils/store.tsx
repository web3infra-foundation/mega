import { CookieValueTypes } from 'cookies-next'
import { atom } from 'jotai'
import { atomFamily } from 'jotai/utils'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

export type IssueIndexFilterType = 'open' | 'closed' | 'Merged' | 'draft'

// export const filterAtom = atomFamily(
//   ({ scope, part }: { scope: CookieValueTypes; part: string }) =>
//     atomWithWebStorage<IssueIndexFilterType>(`${scope}:${part}-index-filter`, 'open'),
//   (a, b) => a.scope === b.scope && a.part === b.part
// )

export const filterAtom = atomFamily(
  ({ part: _part }: { part: string }) => atom<'open' | 'closed'>('open'),
  (a, b) => a.part === b.part
)

export interface IssueSortType {
  [key: string]: string | string[]
}

export const sortAtom = atomFamily(
  ({ scope, filter }: { scope: CookieValueTypes; filter: string }) =>
    atomWithWebStorage<IssueSortType>(`${scope}:issue-index-sort:${filter}`, { Author: '', Assignees: '' }),
  (a, b) => a.scope === b.scope && a.filter === b.filter
)

export const darkModeAtom = atomWithWebStorage<boolean>('darkMode', false)

export const currentPage = atomWithWebStorage<number>('currentPage', 1)

export const issueOpenCurrentPage = atomWithWebStorage('IssueOpencurrentPage', 1)
export const issueCloseCurrentPage = atomWithWebStorage('IssueClosecurrentPage', 1)

export const mrOpenCurrentPage = atomWithWebStorage('MROpencurrentPage', 1)
export const mrCloseCurrentPage = atomWithWebStorage('MRClosecurrentPage', 1)

export const labelsOpenCurrentPage = atomWithWebStorage('LabelsOpenCurrentPage', 1)
export const labelsCloseCurrentPage = atomWithWebStorage('LabelsCloseCurrentPage', 1)

export const idAtom = atom(0)
export const mridAtom = atom(0)

export const FALSE_EDIT_VAL = -1
export const editIdAtom = atom(0)

export const refreshAtom = atom(0)

export const buildIdAtom = atom('')
