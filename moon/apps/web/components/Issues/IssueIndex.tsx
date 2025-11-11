'use client'

import { useState } from 'react'

import { cn } from '@gitmono/ui/src/utils'

import { IndexPageContainer, IndexPageContent } from '@/components/IndexPages/components'
import { IssueBreadcrumbIcon } from '@/components/Titlebar/BreadcrumbPageIcons'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'

import { IssuesContent } from './IssuesContent'
import IssueSearch from './IssueSearch'

export const IssueIndex = () => {
  const [filterQuery, setFilterQuery] = useState('')
  const [shouldClearFilters, setShouldClearFilters] = useState(false)

  const handleClearFilters = () => {
    setShouldClearFilters(true)
  }

  return (
    <>
      <IndexPageContainer>
        <BreadcrumbTitlebar>
          <IssueBreadcrumbIcon />
        </BreadcrumbTitlebar>

        <IndexPageContent
          id='/[org]/issue'
          className={cn('@container', 'max-w-full lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl')}
        >
          <IssueSearch filterQuery={filterQuery} onClearFilters={handleClearFilters} />
          <IssuesContent
            setFilterQuery={setFilterQuery}
            shouldClearFilters={shouldClearFilters}
            setShouldClearFilters={setShouldClearFilters}
          />
        </IndexPageContent>
      </IndexPageContainer>
    </>
  )
}
