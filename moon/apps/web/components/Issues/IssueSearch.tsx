import { useState } from 'react'
import { useDebounce } from 'use-debounce'

import { Button, Link } from '@gitmono/ui'

import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'

import Search from './Search'
import { fuseOptions, searchList } from './utils/consts'

export default function IssueSearch() {
  const [query, setQuery] = useState('')
  const [queryDebounced] = useDebounce(query, 150)
  const [open, setOpen] = useState(false)
  const handleQuery = (val: string) => {
    setOpen(true)
    setQuery(val)
  }

  // const isSearching = query.length > 0
  // const isSearchLoading = queryDebounced.length > 0 && getNotes.isFetching
  const isSearchLoading = queryDebounced.length > 0

  return (
    <>
      <BreadcrumbTitlebar className='z-20 justify-between border-b-0 px-0'>
        <Search
          SearchQuery={{ query, setQuery: handleQuery, isSearchLoading }}
          SearchListTable={{
            open,
            setOpen,
            items: searchList
          }}
          SearchOptions={{
            fuseOptions,
            searchList
          }}
        />
        {/* <IndexSearchInput query={query} setQuery={setQuery} isSearchLoading={isSearchLoading} /> */}
        <Button variant='primary' size={'base'}>
          Labels
        </Button>
        <NewIssueButton />
      </BreadcrumbTitlebar>
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
