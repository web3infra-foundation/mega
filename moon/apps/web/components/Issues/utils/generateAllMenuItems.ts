// import {OrganizationMember as Member} from '@gitmono/types'
import React from 'react'

// import { SyncOrganizationMember as Member } from '@gitmono/types'

export interface MenuConfig<T> {
  key: string
  onSelectFactory: (item: T) => (e: Event) => void
  labelFactory: (item: T) => React.ReactNode
  className?: string
  isChosen: (item: T) => boolean
}

export const generateAllMenuItems = <T,>(members: T[], config: MenuConfig<T>[]) => {
  const result = new Map()

  config.map((c) => result.set(c.key, { chosen: [], all: [] }))

  for (const item of members) {
    for (const { key, onSelectFactory, isChosen, labelFactory, className } of config) {
      const menu = {
        type: 'item' as const,
        label: labelFactory(item),
        className,
        onSelect: onSelectFactory(item)
      }

      if (isChosen(item)) {
        result.get(key).chosen.push(menu)
      } else {
        result.get(key).all.push(menu)
      }
    }
  }

  return result
}
