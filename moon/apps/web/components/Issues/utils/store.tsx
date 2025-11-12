// import { CookieValueTypes } from 'cookies-next'
import { atom } from 'jotai'

// import { atomFamily } from 'jotai/utils'

// import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

// export type IssueIndexFilterType = 'open' | 'closed' | 'Merged' | 'draft'

// export const filterAtom = atomFamily(
//   ({ scope, part }: { scope: CookieValueTypes; part: string }) =>
//     atomWithWebStorage<IssueIndexFilterType>(`${scope}:${part}-index-filter`, 'open'),
//   (a, b) => a.scope === b.scope && a.part === b.part
// )

// export const filterAtom = atomFamily(
//   ({ part: _part }: { part: string }) => atom<'open' | 'closed'>('open'),
//   (a, b) => a.part === b.part
// )

export const issueIdAtom = atom(0)
export const clIdAtom = atom(0)

export const FALSE_EDIT_VAL = -1
export const editIdAtom = atom(0)

export const refreshAtom = atom(0)

export const buildIdAtom = atom('')
