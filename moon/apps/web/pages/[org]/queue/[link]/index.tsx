'use client'

import React, { useEffect, useMemo, useState } from 'react'

import { QueueStats } from '@gitmono/types'
import { QueueStatus } from '@gitmono/types/generated'

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

const POLLING_INTERVAL_MS = 6000

const QueuePage: PageWithLayout<any> = () => {
  const [listPollingEnabled, setListPollingEnabled] = useState(false)

  const { data: queueList, isLoading: listLoading } = useGetMergeQueueList(undefined, {
    refetchInterval: listPollingEnabled ? POLLING_INTERVAL_MS : false
  })

  const queueItems = useMemo(() => {
    return queueList?.data?.items || []
  }, [queueList])

  const hasActiveItems = useMemo(() => {
    return queueItems.some((item) => item.status !== QueueStatus.Merged && item.status !== QueueStatus.Failed)
  }, [queueItems])

  useEffect(() => {
    if (hasActiveItems !== listPollingEnabled) {
      setListPollingEnabled(hasActiveItems)
    }
  }, [hasActiveItems, listPollingEnabled])

  const { data: queueStats, isLoading: statsLoading } = useGetMergeQueueStats(undefined, {
    refetchInterval: listPollingEnabled ? POLLING_INTERVAL_MS : false
  })

  return (
    <QueueView
      // items={list?.data?.items || []}
      // stats={stats.data.stats || defaultStats}
      items={queueItems}
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
