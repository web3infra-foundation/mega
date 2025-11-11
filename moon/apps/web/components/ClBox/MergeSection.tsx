import React from 'react'

import { AlertIcon, CheckCircleIcon, LoadingSpinner, WarningTriangleIcon } from '@gitmono/ui'

interface MergeSectionProps {
  isNowUserApprove?: boolean
  isAllReviewerApproved: boolean
  hasCheckFailures: boolean
  onMerge: () => Promise<void>
  onApprove: () => void
  isMerging: boolean
}

export const MergeSection = React.memo<MergeSectionProps>(
  ({ isAllReviewerApproved, hasCheckFailures, isNowUserApprove, onMerge, onApprove, isMerging }) => {
    let statusNode: React.ReactNode
    const isMergeable = isAllReviewerApproved && !hasCheckFailures

    if (!isAllReviewerApproved) {
      statusNode = (
        <div className='flex items-center text-yellow-700'>
          <WarningTriangleIcon className='mr-3 h-5 w-5' />
          <span className='font-semibold'>Merging is blocked</span>
        </div>
      )
    } else if (hasCheckFailures) {
      statusNode = (
        <div className='flex items-center text-red-700'>
          <AlertIcon className='mr-3 h-5 w-5' />
          <span className='font-semibold'>Merging is blocked</span>
        </div>
      )
    } else {
      statusNode = (
        <div className='flex items-center text-green-700'>
          <CheckCircleIcon className='mr-3 h-5 w-5' />
          <span className='font-semibold'>Allow merging</span>
        </div>
      )
    }

    return (
      <div className='p-3'>
        {statusNode}
        <div className='ClBox-MergeSection flex items-center justify-center gap-4' style={{ marginTop: '12px' }}>
          <button
            onClick={onApprove}
            disabled={isNowUserApprove === undefined || isNowUserApprove}
            className='w-full rounded-md bg-green-600 px-4 py-2 font-bold text-white duration-500 hover:bg-green-800 disabled:cursor-not-allowed disabled:bg-gray-400'
          >
            Approve
          </button>
          <button
            onClick={onMerge}
            disabled={!isMergeable}
            className='w-full rounded-md bg-green-600 px-4 py-2 font-bold text-white duration-500 hover:bg-green-800 disabled:cursor-not-allowed disabled:bg-gray-400'
          >
            {isMerging ? <LoadingSpinner /> : 'Confirm Merge'}
          </button>
        </div>
      </div>
    )
  }
)

MergeSection.displayName = 'MergeSection'
