import { useEffect } from 'react'
import { RESET } from 'jotai/utils'
import { useRouter } from 'next/router'

import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

import { MuiTreeNode } from './TreeUtils'

export const treeAllDataAtom = atomWithWebStorage<MuiTreeNode[]>('treeAllDataAtom', [])

export const expandedNodesAtom = atomWithWebStorage<string[]>('expandedNodes', [])

export function useClearTreeAtoms(setTreeAllData: (v: any) => void, setExpandedNodes: (v: any) => void) {
  const router = useRouter()

  useEffect(() => {
    if (!router.asPath.startsWith(`/${router.query.org}/code`)) {
      setTreeAllData(RESET)
      setExpandedNodes(RESET)
    }
  }, [router.asPath, router.query.org, setTreeAllData, setExpandedNodes])
}
