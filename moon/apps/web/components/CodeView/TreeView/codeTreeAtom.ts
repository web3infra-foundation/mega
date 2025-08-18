import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { MuiTreeNode } from './TreeUtils'


export const treeAllDataAtom = atomWithWebStorage<MuiTreeNode[]>('treeAllDataAtom', [])

export const expandedNodesAtom = atomWithWebStorage<string[]>('expandedNodes', [])
