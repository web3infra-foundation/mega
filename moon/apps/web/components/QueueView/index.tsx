import React from 'react'

import type { QueueItem, QueueStats } from '@gitmono/types/generated'
import { cn } from '@gitmono/ui'

import { Heading } from '@/components/ClView/catalyst/heading'
import { IndexPageContainer, IndexPageContent } from '@/components/IndexPages/components'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'

import { QueueItemsList } from './items'
import { QueueStatsCard } from './stats'

interface QueueViewProps {
  items: QueueItem[]
  stats: QueueStats
  isLoading?: boolean
}

export const QueueView: React.FC<QueueViewProps> = ({ items, stats, isLoading = false }) => {
  return (
    <IndexPageContainer>
      <BreadcrumbTitlebar>
        <Heading>Merge queue</Heading>
      </BreadcrumbTitlebar>

      <IndexPageContent
        id='/[org]/queue'
        className={cn('@container', 'max-w-full lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl')}
      >
        <div className='grid grid-cols-1 gap-6 md:grid-cols-4'>
          <div className='md:col-span-3'>
            <QueueItemsList items={items} stats={stats} isLoading={isLoading} />
          </div>

          <div className='md:col-span-1'>
            <QueueStatsCard stats={stats} isLoading={isLoading} />
          </div>
        </div>
      </IndexPageContent>
    </IndexPageContainer>
  )
}
