'use client'

import React from 'react'

import { QueueStats } from '@gitmono/types'

import { AppLayout } from '@/components/Layout/AppLayout'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { QueueView } from '@/components/QueueView'
import { useGetMergeQueueList } from '@/hooks/MergeQueue/useGetMergeQueueList'
import { useGetMergeQueueStats } from '@/hooks/MergeQueue/useGetMergeQueueStats'
import { PageWithLayout } from '@/utils/types'

// import { stats,list } from '@/components/QueueView/mock'

const defaultStats: QueueStats = {
  total_items: 0,
  failed_count: 0,
  merged_count: 0,
  merging_count: 0,
  testing_count: 0,
  waiting_count: 0
}

const QueuePage: PageWithLayout<any> = () => {
  const { data: queueList, isLoading: listLoading } = useGetMergeQueueList()
  const { data: queueStats, isLoading: statsLoading } = useGetMergeQueueStats()

  return (
    <QueueView
      // items={list?.data?.items || []}
      // stats={stats.data.stats || defaultStats}
      items={queueList?.data?.items || []}
      stats={queueStats?.data?.stats || defaultStats}
      ListLoading={listLoading}
      StatsisLoading={statsLoading}
    />
  )
}

QueuePage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default QueuePage
