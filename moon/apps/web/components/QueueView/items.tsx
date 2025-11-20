import React, { useMemo, useState } from 'react'
import { GitMergeIcon, GitPullRequestClosedIcon, GitPullRequestIcon } from '@primer/octicons-react'
import { formatDistanceToNow } from 'date-fns'

import type { QueueItem, QueueStats } from '@gitmono/types/generated'
import { QueueStatus } from '@gitmono/types/generated'
import { Button } from '@gitmono/ui'
import * as Dialog from '@gitmono/ui/src/Dialog'

import { ItemLabels, ListItem, List as QueueList } from '@/components/ClView/ClList'
import { BreadcrumbTitlebarContainer } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useDeleteMergeQueueRemove } from '@/hooks/MergeQueue/useDeleteMergeQueueRemove'
import { usePostMergeQueueCancelAll } from '@/hooks/MergeQueue/usePostMergeQueueCancelAll'
import { usePostMergeQueueRetry } from '@/hooks/MergeQueue/usePostMergeQueueRetry'

interface QueueItemsListProps {
  items: QueueItem[]
  stats: QueueStats
  isLoading?: boolean
}

const getStatusIcon = (status: QueueStatus) => {
  switch (status) {
    case QueueStatus.Waiting:
      return <GitPullRequestIcon className='text-[#378f50]' />
    case QueueStatus.Testing:
      return <GitPullRequestIcon className='text-[#2f81f7]' />
    case QueueStatus.Merging:
      return <GitMergeIcon className='text-[#986ee2]' />
    case QueueStatus.Merged:
      return <GitMergeIcon className='text-gray-500' />
    case QueueStatus.Failed:
      return <GitPullRequestClosedIcon className='text-[#d1242f]' />
    default:
      return null
  }
}

export const QueueItemsList: React.FC<QueueItemsListProps> = ({ items, stats, isLoading = false }) => {
  const { mutate: retryQueue, isPending: isRetrying } = usePostMergeQueueRetry()
  const { mutate: removeFromQueue, isPending: isRemoving } = useDeleteMergeQueueRemove()
  const { mutate: cancelAll, isPending: isCancellingAll } = usePostMergeQueueCancelAll()

  const [confirmDialogOpen, setConfirmDialogOpen] = useState(false)
  const [confirmCancelAllOpen, setConfirmCancelAllOpen] = useState(false)
  const [selectedClLink, setSelectedClLink] = useState<string>('')

  const sortedItems = useMemo(() => {
    return [...items].sort((a, b) => {
      const posA = a.display_position ?? Number.MAX_SAFE_INTEGER
      const posB = b.display_position ?? Number.MAX_SAFE_INTEGER

      return posA - posB
    })
  }, [items])

  const cancellableCount = useMemo(() => {
    return sortedItems.filter((item) => item.status !== QueueStatus.Merging && item.status !== QueueStatus.Merged)
      .length
  }, [sortedItems])

  const handleRetry = (clLink: string, e: React.MouseEvent) => {
    e.stopPropagation()
    retryQueue(clLink)
  }

  const handleCancelClick = (clLink: string, e: React.MouseEvent) => {
    e.stopPropagation()
    setSelectedClLink(clLink)
    setConfirmDialogOpen(true)
  }

  const handleConfirmRemove = () => {
    if (selectedClLink) {
      removeFromQueue(selectedClLink)
      setConfirmDialogOpen(false)
      setSelectedClLink('')
    }
  }

  const handleCancelAllClick = () => {
    setConfirmCancelAllOpen(true)
  }

  const handleConfirmCancelAll = () => {
    cancelAll({})
    setConfirmCancelAllOpen(false)
  }

  const header = (
    <BreadcrumbTitlebarContainer className='justify-between bg-gray-100 pl-3 pr-3'>
      <div className='flex items-center gap-3'>
        <span className='p-2 font-medium'>{sortedItems.length} items</span>
        <span className='text-sm text-gray-500'>
          Waiting {stats.waiting_count} · Testing {stats.testing_count} · Merging {stats.merging_count}
        </span>
      </div>

      <div className='flex items-center gap-2'>
        <button
          className='rounded-md bg-gray-200 px-2 py-1 text-sm text-gray-700 hover:bg-gray-300 disabled:cursor-not-allowed disabled:opacity-50'
          onClick={handleCancelAllClick}
          disabled={isCancellingAll || cancellableCount === 0}
        >
          Cancel all
        </button>
      </div>
    </BreadcrumbTitlebarContainer>
  )

  return (
    <>
      <QueueList isLoading={isLoading} lists={sortedItems} header={header}>
        {(lists) =>
          lists.map((queueItem) => {
            const labels = <ItemLabels item={{ labels: [] } as any} />

            const subline = (
              <div className='mt-1 space-y-1 text-sm text-gray-500'>
                {queueItem.status !== QueueStatus.Failed && (
                  <div className='flex flex-wrap items-center gap-2'>
                    <span className='font-medium'>{queueItem.status}</span>
                    <span>started {formatDistanceToNow(new Date(queueItem.updated_at), { addSuffix: true })}</span>

                    {queueItem.retry_count > 0 && <span>• {queueItem.retry_count} retries</span>}
                  </div>
                )}

                {queueItem.status === QueueStatus.Failed && queueItem.error && (
                  <>
                    <div className='flex flex-wrap items-center gap-2'>
                      <span className='font-medium'>{queueItem.status}</span>

                      <span>{formatDistanceToNow(new Date(queueItem.error.occurred_at), { addSuffix: true })}</span>

                      {queueItem.retry_count > 0 && <span>• {queueItem.retry_count} retries</span>}
                    </div>

                    <div className='text-sm text-red-500'>
                      <div
                        className='max-w-md cursor-help truncate leading-snug'
                        title={`${queueItem.error.failure_type}: ${queueItem.error.message}`}
                      >
                        <span className='font-medium'>{queueItem.error.failure_type}</span>
                        <span>: {queueItem.error.message}</span>
                      </div>
                    </div>
                  </>
                )}
              </div>
            )

            const right = (
              <div className='flex items-center gap-4 text-sm text-gray-500'>
                <div className='flex gap-2'>
                  {queueItem.status === QueueStatus.Merging && (
                    <div className='flex items-center pr-5'>
                      <div className='h-4 w-4 animate-spin rounded-full border-2 border-solid border-blue-500 border-t-transparent'></div>
                    </div>
                  )}

                  {queueItem.status === QueueStatus.Failed && queueItem.retry_count < 3 && (
                    <button
                      className='rounded-md bg-blue-500 px-2 py-1 text-xs text-white hover:bg-blue-600 disabled:opacity-50'
                      onClick={(e) => handleRetry(queueItem.cl_link, e)}
                      disabled={isRetrying}
                    >
                      Retry
                    </button>
                  )}

                  {queueItem.status !== QueueStatus.Merging && queueItem.status !== QueueStatus.Merged && (
                    <button
                      className='rounded-md bg-gray-200 px-2 py-1 text-xs text-gray-700 hover:bg-gray-300 disabled:opacity-50'
                      onClick={(e) => handleCancelClick(queueItem.cl_link, e)}
                      disabled={isRemoving}
                    >
                      Cancel
                    </button>
                  )}
                </div>
              </div>
            )

            return (
              <ListItem
                key={queueItem.cl_link}
                title={queueItem.cl_link}
                leftIcon={getStatusIcon(queueItem.status)}
                labels={labels}
                rightIcon={right}
              >
                {subline}
              </ListItem>
            )
          })
        }
      </QueueList>

      <Dialog.Root open={confirmDialogOpen} onOpenChange={setConfirmDialogOpen} size='sm'>
        <Dialog.Header>
          <Dialog.Title>Remove from queue</Dialog.Title>
          <Dialog.Description>Are you sure you want to remove this CL from the queue?</Dialog.Description>
        </Dialog.Header>

        <Dialog.Footer>
          <Dialog.TrailingActions>
            <Button variant='flat' onClick={() => setConfirmDialogOpen(false)} disabled={isRemoving}>
              Cancel
            </Button>
            <Button
              variant='destructive'
              onClick={handleConfirmRemove}
              disabled={isRemoving}
              loading={isRemoving}
              autoFocus
            >
              Remove
            </Button>
          </Dialog.TrailingActions>
        </Dialog.Footer>
      </Dialog.Root>

      <Dialog.Root open={confirmCancelAllOpen} onOpenChange={setConfirmCancelAllOpen} size='sm'>
        <Dialog.Header>
          <Dialog.Title>Cancel all items</Dialog.Title>
          <Dialog.Description>
            Are you sure you want to cancel all items in the queue? This will remove {cancellableCount}{' '}
            {cancellableCount === 1 ? 'item' : 'items'} (excluding items currently merging).
          </Dialog.Description>
        </Dialog.Header>

        <Dialog.Footer>
          <Dialog.TrailingActions>
            <Button variant='flat' onClick={() => setConfirmCancelAllOpen(false)} disabled={isCancellingAll}>
              Cancel
            </Button>
            <Button
              variant='destructive'
              onClick={handleConfirmCancelAll}
              disabled={isCancellingAll}
              loading={isCancellingAll}
              autoFocus
            >
              Cancel all
            </Button>
          </Dialog.TrailingActions>
        </Dialog.Footer>
      </Dialog.Root>
    </>
  )
}
