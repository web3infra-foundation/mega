'use client'

import { useState } from 'react'
import { useAtom } from 'jotai'

// import { useDebounce } from 'use-debounce'

import { Button, LayeredHotkeys, Link, UIText } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import { FloatingNewDocButton } from '@/components/FloatingButtons/NewDoc'
import { IndexPageContainer, IndexPageContent, IndexPageEmptyState } from '@/components/IndexPages/components'
import { SplitViewContainer, SplitViewDetail } from '@/components/SplitView'
import { IssueBreadcrumbIcon } from '@/components/Titlebar/BreadcrumbPageIcons'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'

import { IssuesContent } from './IssuesContent'
import IssueSearch from './IssueSearch'
import { filterAtom } from './utils/store'

export const IssueIndex = () => {
  const [query, _setQuery] = useState('')
  // const [queryDebounced] = useDebounce(query, 150)

  const isSearching = query.length > 0
  // const isSearchLoading = queryDebounced.length > 0 && getNotes.isFetching
  // const isSearchLoading = queryDebounced.length > 0

  return (
    <>
      <FloatingNewDocButton />
      <SplitViewContainer>
        <IndexPageContainer>
          <BreadcrumbTitlebar className='justify-between'>
            <IssueBreadcrumbIcon />
          </BreadcrumbTitlebar>
          <IndexPageContent id='/[org]/issue' className={cn('@container', '3xl:max-w-7xl max-w-7xl')}>
            <IssueSearch />
            <IssuesContent searching={isSearching} />
          </IndexPageContent>
        </IndexPageContainer>
        <SplitViewDetail />
      </SplitViewContainer>
    </>
  )
}

export function IssueIndexTabFilter({
  part,
  fullWidth = false,
  openNum,
  closeNum,
  openTooltip,
  closeTooltip
}: {
  part: string
  fullWidth?: boolean
  openNum?: number
  closeNum?: number
  openTooltip?: string
  closeTooltip?: string
}) {
  const { scope } = useScope()

  const [filter, setFilter] = useAtom(filterAtom({ scope, part: `${part}` }))

  return (
    <>
      <LayeredHotkeys keys='1' callback={() => setFilter('open')} />
      <LayeredHotkeys keys='2' callback={() => setFilter('closed')} />

      <Button
        size='sm'
        fullWidth={fullWidth}
        onClick={() => setFilter('open')}
        variant={filter === 'open' ? 'flat' : 'plain'}
        tooltip={openTooltip}
      >
        Open {openNum}
      </Button>
      <Button
        size='sm'
        fullWidth={fullWidth}
        onClick={() => setFilter('closed')}
        variant={filter === 'closed' ? 'flat' : 'plain'}
        tooltip={closeTooltip}
      >
        Closed {closeNum}
      </Button>
    </>
  )
}

export const NewIssueButton = () => {
  const { scope } = useScope()

  return (
    <Link href={`/${scope}/issue/new`}>
      <Button variant='primary' className='bg-[#1f883d]' size={'base'}>
        New Issue
      </Button>
    </Link>
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
