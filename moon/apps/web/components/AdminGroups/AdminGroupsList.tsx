import React from 'react'

import { LoadingSpinner } from '@gitmono/ui'

import { AdminGroupItem } from './AdminGroupItem'

interface GroupItem {
  id: number
  name: string
  description?: string | null
  created_at: number
  updated_at: number
}

interface AdminGroupsListProps {
  groups: GroupItem[]
  total: number
  isLoading: boolean
  isError: boolean
  onDelete: (id: number) => void
  onManageMembers: (id: number) => void
  onEdit?: (id: number) => void
}

export const AdminGroupsList = ({
  groups,
  total,
  isLoading,
  isError,
  onDelete,
  onManageMembers,
  onEdit
}: AdminGroupsListProps) => {
  return (
    <section>
      <h2 className='border-primary text-primary border-b pb-2 text-xl font-semibold'>Groups ({total})</h2>
      {isLoading ? (
        <div className='flex h-[200px] items-center justify-center'>
          <LoadingSpinner />
        </div>
      ) : isError ? (
        <div className='flex h-[200px] items-center justify-center'>
          <p className='text-red-500'>Failed to load groups list</p>
        </div>
      ) : groups.length === 0 ? (
        <div className='flex h-[200px] items-center justify-center'>
          <p className='text-tertiary'>No groups found</p>
        </div>
      ) : (
        <div>
          {groups.map((group: GroupItem) => (
            <AdminGroupItem
              key={group.id}
              group={group}
              onDelete={onDelete}
              onManageMembers={onManageMembers}
              onEdit={onEdit}
            />
          ))}
        </div>
      )}
    </section>
  )
}
