'use client'

import { type ReactNode, createContext, useRef, useContext } from 'react'
import { type StoreApi, useStore } from 'zustand'

import { type TreeStore, createTreeStore, initTreeStore } from '@/stores/tree-store'

export const TreeStoreContext = createContext<StoreApi<TreeStore> | null>(
  null,
)

export interface TreeStoreProviderProps {
  children: ReactNode
}

export const TreeStoreProvider = ({
  children,
}: TreeStoreProviderProps) => {
  const storeRef = useRef<StoreApi<TreeStore>>()
  if (!storeRef.current) {
    storeRef.current = createTreeStore(initTreeStore())
  }

  return (
    <TreeStoreContext.Provider value={storeRef.current}>
      {children}
    </TreeStoreContext.Provider>
  )
}

export const useTreeStore = <T,>(
  selector: (store: TreeStore) => T,
): T => {
  const treeStoreContext = useContext(TreeStoreContext)

  if (!treeStoreContext) {
    throw new Error(`useTreeStore must be use within TreeStoreProvider`)
  }

  return useStore(treeStoreContext, selector)
}
