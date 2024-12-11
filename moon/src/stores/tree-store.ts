import { createStore } from 'zustand/vanilla'
import { persist, createJSONStorage } from 'zustand/middleware'

export type TreeState = {
  expandedKeys: string[]
}

export type TreeActions = {
  setExpandedKeys: (expandedKeys: string[]) => void
}

export type TreeStore = TreeState & TreeActions

export const initTreeStore = (): TreeState => {
  return {
    expandedKeys: [] as string[],
  }
}

export const defaultInitState: TreeState = {
  expandedKeys: [] as string[],
}

export const createTreeStore = (
  initState: TreeState = defaultInitState,
) => {
  return createStore<TreeStore>()(
    persist(
      (set) => ({
        ...initState,
        setExpandedKeys: (expandedKeysValue: string[]) => set(() => ({
          expandedKeys: expandedKeysValue,
        })),
      }),
      {
        name: 'mega-tree-storage', // name of the item in the storage (must be unique)
        storage: createJSONStorage(() => localStorage), // (optional) by default, 'localStorage' is used
        skipHydration: true,
      },
    )
  )
}
