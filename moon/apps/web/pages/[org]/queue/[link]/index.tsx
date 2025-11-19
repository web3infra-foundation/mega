'use client'

import React from 'react'

import type { QueueItem, QueueStats } from '@gitmono/types/generated'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { QueueView } from '@/components/QueueView'
import { list, stats } from '@/components/QueueView/mook'
// import { useDeleteMergeQueueRemove } from '@/hooks/MergeQueue/useDeleteMergeQueueRemove'
// import { usePostMergeQueueCancelAll } from '@/hooks/MergeQueue/usePostMergeQueueCancelAll'
// import { usePostMergeQueueRetry } from '@/hooks/MergeQueue/usePostMergeQueueRetry'

// import { useGetMergeQueueList } from '@/hooks/MergeQueue/useGetMergeQueueList'
// import { useGetMergeQueueStats } from '@/hooks/MergeQueue/useGetMergeQueueStats'

import { PageWithLayout } from '@/utils/types'

//  mock
const mockQueueItems: QueueItem[] = list.data.items as QueueItem[]
const mockStats: QueueStats = stats.data.stats as QueueStats

const isLoading = false

const QueuePage: PageWithLayout<any> = () => {
  return <QueueView items={mockQueueItems} stats={mockStats} isLoading={isLoading} />
}

QueuePage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default QueuePage
