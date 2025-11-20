import React from 'react'
import {
  ClockIcon,
  GitMergeIcon,
  GitPullRequestClosedIcon,
  GitPullRequestIcon,
  XCircleIcon
} from '@primer/octicons-react'

import type { QueueStats } from '@gitmono/types/generated'

interface QueueStatsCardProps {
  stats: QueueStats
  isLoading?: boolean
}

export const QueueStatsCard: React.FC<QueueStatsCardProps> = ({ stats, isLoading }) => {
  if (isLoading) {
    return (
      <div className='rounded-lg border border-gray-200 bg-white p-4 shadow-sm'>
        <div className='animate-pulse'>
          <div className='mb-3 h-4 w-32 rounded bg-gray-200'></div>
          <div className='space-y-3'>
            <div className='h-8 w-full rounded bg-gray-200'></div>
            <div className='h-8 w-full rounded bg-gray-200'></div>
            <div className='h-8 w-full rounded bg-gray-200'></div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className='flex flex-col gap-3'>
      <div className='rounded-lg border border-gray-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md'>
        <div className='mb-3 flex items-center justify-between'>
          <span className='text-xs font-semibold uppercase tracking-wide text-gray-500'>Queue Overview</span>
        </div>

        <div className='flex items-center gap-3'>
          <div className='flex h-12 w-12 items-center justify-center rounded-full bg-gray-100'>
            <GitMergeIcon size={20} className='text-gray-600' />
          </div>
          <div className='flex flex-col'>
            <span className='text-2xl font-bold text-gray-900'>{stats.total_items}</span>
            <span className='text-xs text-gray-500'>Items in queue</span>
          </div>
        </div>
      </div>

      <div className='rounded-lg border border-gray-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md'>
        <span className='mb-3 block text-xs font-semibold uppercase tracking-wide text-gray-500'>Status Breakdown</span>

        <div className='space-y-2.5'>
          {/* Waiting */}
          <div className='group flex items-center justify-between rounded-md p-2 transition-colors hover:bg-yellow-50'>
            <div className='flex items-center gap-2.5'>
              <div className='flex h-8 w-8 items-center justify-center rounded-full bg-yellow-100'>
                <ClockIcon size={14} className='text-yellow-600' />
              </div>
              <span className='text-sm font-medium text-gray-700'>Waiting</span>
            </div>
            <span className='text-lg font-semibold text-yellow-600'>{stats.waiting_count}</span>
          </div>

          {/* Testing */}
          <div className='group flex items-center justify-between rounded-md p-2 transition-colors hover:bg-blue-50'>
            <div className='flex items-center gap-2.5'>
              <div className='flex h-8 w-8 items-center justify-center rounded-full bg-blue-100'>
                <GitPullRequestIcon size={14} className='text-blue-600' />
              </div>
              <span className='text-sm font-medium text-gray-700'>Testing</span>
            </div>
            <span className='text-lg font-semibold text-blue-600'>{stats.testing_count}</span>
          </div>

          {/* Merging */}
          <div className='group flex items-center justify-between rounded-md p-2 transition-colors hover:bg-purple-50'>
            <div className='flex items-center gap-2.5'>
              <div className='flex h-8 w-8 items-center justify-center rounded-full bg-purple-100'>
                <GitMergeIcon size={14} className='text-purple-600' />
              </div>
              <span className='text-sm font-medium text-gray-700'>Merging</span>
            </div>
            <span className='text-lg font-semibold text-purple-600'>{stats.merging_count}</span>
          </div>

          {/* Failed */}
          <div className='group flex items-center justify-between rounded-md p-2 transition-colors hover:bg-red-50'>
            <div className='flex items-center gap-2.5'>
              <div className='flex h-8 w-8 items-center justify-center rounded-full bg-red-100'>
                <XCircleIcon size={14} className='text-red-600' />
              </div>
              <span className='text-sm font-medium text-gray-700'>Failed</span>
            </div>
            <span className='text-lg font-semibold text-red-600'>{stats.failed_count}</span>
          </div>

          {/* Merged */}
          <div className='group flex items-center justify-between rounded-md p-2 transition-colors hover:bg-green-50'>
            <div className='flex items-center gap-2.5'>
              <div className='flex h-8 w-8 items-center justify-center rounded-full bg-green-100'>
                <GitPullRequestClosedIcon size={14} className='text-green-600' />
              </div>
              <span className='text-sm font-medium text-gray-700'>Merged</span>
            </div>
            <span className='text-lg font-semibold text-green-600'>{stats.merged_count}</span>
          </div>
        </div>
      </div>
    </div>
  )
}
