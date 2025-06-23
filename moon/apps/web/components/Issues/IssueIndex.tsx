'use client'

import { useState } from 'react'
import { useAtom } from 'jotai'
import { useRouter } from 'next/router'
import { useDebounce } from 'use-debounce'

import { Button, LayeredHotkeys, UIText } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import { FloatingNewDocButton } from '@/components/FloatingButtons/NewDoc'
import {
  IndexPageContainer,
  IndexPageContent,
  IndexPageEmptyState,
  IndexSearchInput
} from '@/components/IndexPages/components'
import { SplitViewContainer, SplitViewDetail } from '@/components/SplitView'
import { IssueBreadcrumbIcon } from '@/components/Titlebar/BreadcrumbPageIcons'
import { BreadcrumbLabel, BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'

import { IssuesContent } from './IssuesContent'
import { filterAtom } from './utils/store'

export const IssueIndex = () => {
  const [query, setQuery] = useState('')
  const [queryDebounced] = useDebounce(query, 150)

  const isSearching = query.length > 0
  // const isSearchLoading = queryDebounced.length > 0 && getNotes.isFetching
  const isSearchLoading = queryDebounced.length > 0

  return (
    <>
      <FloatingNewDocButton />
      <SplitViewContainer>
        <IndexPageContainer>
          <BreadcrumbTitlebar className='justify-between'>
            <IssueBreadcrumbIcon />
            <BreadcrumbLabel>Issue</BreadcrumbLabel>
            <IndexSearchInput query={query} setQuery={setQuery} isSearchLoading={isSearchLoading} />

            <Button variant='primary' size={'base'}>
              Labels
            </Button>
            <NewIssueButton />
          </BreadcrumbTitlebar>
          <IndexPageContent id='/[org]/issue' className={cn('@container', '3xl:max-w-7xl max-w-7xl')}>
            <IssuesContent searching={isSearching} />
          </IndexPageContent>
        </IndexPageContainer>
        <SplitViewDetail />
      </SplitViewContainer>
    </>
  )
}

export function IssueIndexTabFilter({
  fullWidth = false,
  openNum,
  closeNum
}: {
  fullWidth?: boolean
  openNum?: number
  closeNum?: number
}) {
  const { scope } = useScope()

  const [filter, setFilter] = useAtom(filterAtom(scope))

  return (
    <>
      <LayeredHotkeys keys='1' callback={() => setFilter('open')} />
      <LayeredHotkeys keys='2' callback={() => setFilter('closed')} />

      <Button
        size='sm'
        fullWidth={fullWidth}
        onClick={() => setFilter('open')}
        variant={filter === 'open' ? 'flat' : 'plain'}
        tooltip='Issues that are still open and need attention'
      >
        Open {openNum}
      </Button>
      <Button
        size='sm'
        fullWidth={fullWidth}
        onClick={() => setFilter('closed')}
        variant={filter === 'closed' ? 'flat' : 'plain'}
        tooltip='Closed'
      >
        Closed {closeNum}
      </Button>
    </>
  )
}

export const NewIssueButton = () => {
  const router = useRouter()
  const { scope } = useScope()

  return (
    <Button
      variant='primary'
      className='bg-[#1f883d]'
      size={'base'}
      onClick={() => {
        router.push(`/${scope}/issue/new`)
      }}
    >
      New Issue
    </Button>
  )
}

export function IssueIndexEmptyState() {
  return (
    <IndexPageEmptyState>
      <div className='flex flex-col gap-1'>
        <UIText size='text-base' weight='font-semibold'>
          No results
        </UIText>
        <UIText size='text-base' tertiary>
          Try adjusting your search filters.
        </UIText>
      </div>
    </IndexPageEmptyState>
  )
}
