import React from 'react'

import { UsersIcon } from '@gitmono/ui'

interface GroupItem {
  id: number
  name: string
  description?: string | null
  created_at: number
  updated_at: number
}

interface AdminGroupItemProps {
  group: GroupItem
  onDelete: (id: number) => void
  onManageMembers: (id: number) => void
  onEdit?: (id: number) => void
}

export const AdminGroupItem = ({ group, onDelete, onManageMembers, onEdit }: AdminGroupItemProps) => {
  const createdDate = new Date(group.created_at * 1000).toLocaleDateString()

  return (
    <div className='border-primary flex items-center justify-between border-b py-6 transition-colors duration-200 last:border-b-0 hover:bg-gray-50 dark:hover:bg-gray-800/50'>
      <div className='flex items-start'>
        <div className='mr-4 rounded-lg bg-blue-100 p-3 dark:bg-blue-900/30'>
          <UsersIcon className='h-6 w-6 text-blue-600 dark:text-blue-400' aria-hidden='true' />
        </div>
        <div>
          <p className='text-primary mb-1 text-lg font-bold'>{group.name}</p>
          <div className='text-tertiary space-y-1 text-sm'>
            {group.description && <p className='max-w-md text-gray-600 dark:text-gray-400'>{group.description}</p>}
            <p className='text-xs text-gray-500 dark:text-gray-500'>Created: {createdDate}</p>
          </div>
        </div>
      </div>

      <div className='flex min-w-0 flex-col gap-3'>
        {/* Group operations - first row */}
        <div className='flex items-center gap-4'>
          <div className='w-[30%]'>
            <span className='text-sm font-medium text-gray-600 dark:text-gray-400'>Group:</span>
          </div>
          <div className='flex w-[70%] justify-between gap-2'>
            <button
              className='border-primary flex-1 rounded-md border px-4 py-1 text-sm font-semibold text-blue-600 transition-colors duration-200 hover:bg-blue-600 hover:text-white'
              onClick={() => onEdit?.(group.id)}
            >
              Edit
            </button>
            <button
              className='border-primary flex-1 rounded-md border px-4 py-1 text-sm font-semibold text-red-500 transition-colors duration-200 hover:bg-red-500 hover:text-white'
              onClick={() => onDelete(group.id)}
            >
              Delete
            </button>
          </div>
        </div>

        {/* Member operations - second row */}
        <div className='flex items-center gap-4'>
          <div className='w-[30%]'>
            <span className='text-sm font-medium text-gray-600 dark:text-gray-400'>Members:</span>
          </div>
          <div className='flex w-[70%] justify-between'>
            <button
              className='border-primary w-full rounded-md border px-4 py-1 text-sm font-semibold text-purple-600 transition-colors duration-200 hover:bg-purple-600 hover:text-white'
              onClick={() => onManageMembers(group.id)}
            >
              Manage Members
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
