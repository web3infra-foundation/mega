import React from 'react'
import { useRouter } from 'next/router'

import { AlertIcon, CheckCircleIcon, LoadingSpinner, WarningTriangleIcon } from '@gitmono/ui'

import { useScope } from '@/contexts/scope'
import { useGetMergeQueueStatus } from '@/hooks/MergeQueue/useGetMergeQueueStatus'
import { usePostMergeQueueAdd } from '@/hooks/MergeQueue/usePostMergeQueueAdd'

interface MergeSectionProps {
  isNowUserApprove?: boolean
  isAllReviewerApproved: boolean
  hasCheckFailures: boolean
  onMerge: () => Promise<void>
  onApprove: () => void
  isMerging: boolean
  clLink: string
}

export const MergeSection = React.memo<MergeSectionProps>(
  ({ isAllReviewerApproved, hasCheckFailures, isNowUserApprove, onMerge, onApprove, isMerging, clLink }) => {
    const router = useRouter()
    const { scope } = useScope()
    const { data: queueStatusData } = useGetMergeQueueStatus(clLink)
    const { mutate: addToQueue, isPending: isAddingToQueue } = usePostMergeQueueAdd()

    const inQueue = queueStatusData?.data?.in_queue ?? false
    const queueItem = queueStatusData?.data?.item

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

    const handleAddToQueue = () => {
      addToQueue({ cl_link: clLink })
    }

    const handleViewQueue = () => {
      router.push(`/${scope}/queue/main`)
    }

    return (
      <div className='p-3'>
        {statusNode}

        {/* Queue Status Info */}
        {inQueue && queueItem && (
          <div className='mb-3 mt-2 rounded-md bg-blue-50 p-2 text-sm'>
            <div className='flex items-center justify-between'>
              <span className='text-blue-800'>
                In merge queue • Status: <strong>{queueItem.status}</strong>
              </span>
              <button onClick={handleViewQueue} className='text-blue-600 underline hover:text-blue-800'>
                View Queue
              </button>
            </div>
          </div>
        )}

        <div className='ClBox-MergeSection flex items-center justify-center gap-4' style={{ marginTop: '12px' }}>
          <button
            onClick={onApprove}
            disabled={isNowUserApprove === undefined || isNowUserApprove}
            className='w-full rounded-md bg-green-600 px-4 py-2 font-bold text-white duration-500 hover:bg-green-800 disabled:cursor-not-allowed disabled:bg-gray-400'
          >
            Approve
          </button>

          {!inQueue ? (
            <button
              onClick={handleAddToQueue}
              disabled={isAddingToQueue || !isMergeable}
              className='w-full rounded-md bg-purple-600 px-4 py-2 font-bold text-white duration-500 hover:bg-purple-800 disabled:cursor-not-allowed disabled:bg-gray-400'
            >
              {isAddingToQueue ? <LoadingSpinner /> : 'Add to Queue'}
            </button>
          ) : (
            <button
              onClick={onMerge}
              disabled={!isMergeable}
              className='w-full rounded-md bg-green-600 px-4 py-2 font-bold text-white duration-500 hover:bg-green-800 disabled:cursor-not-allowed disabled:bg-gray-400'
            >
              {isMerging ? <LoadingSpinner /> : 'Merge Now'}
            </button>
          )}
        </div>
      </div>
    )
  }
)

MergeSection.displayName = 'MergeSection'
