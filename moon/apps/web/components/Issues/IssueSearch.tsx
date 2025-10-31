// import { useState } from 'react'
// import { useDebounce } from 'use-debounce'

import { XIcon } from '@primer/octicons-react'

import { Button, Link } from '@gitmono/ui'

import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'

// import Search from './Search'
// import { fuseOptions, searchList } from './utils/consts'

interface IssueSearchProps {
  filterQuery?: string
  onClearFilters?: () => void
}

export default function IssueSearch({ filterQuery, onClearFilters }: IssueSearchProps) {
  // const [query, setQuery] = useState('')
  // const [queryDebounced] = useDebounce(query, 150)
  // const [open, setOpen] = useState(false)
  // const handleQuery = (val: string) => {
  //   setOpen(true)
  //   setQuery(val)
  // }

  // const isSearching = query.length > 0
  // const isSearchLoading = queryDebounced.length > 0 && getNotes.isFetching
  // const isSearchLoading = queryDebounced.length > 0

  return (
    <>
      <BreadcrumbTitlebar className='z-20 justify-between border-b-0 px-0'>
        {/* <Search
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
        /> */}

        <div className='relative flex flex-1 items-center'>
          <input
            type='text'
            value={filterQuery || ''}
            readOnly
            placeholder='Filter issues by author, assignee, or labels...'
            className='flex-1 rounded-md border border-gray-300 bg-gray-50 px-3 py-2 pr-10 text-sm text-gray-700'
          />
          {filterQuery && (
            <button
              onClick={onClearFilters}
              className='absolute right-2 flex items-center justify-center rounded-md p-1 text-gray-400 transition-all hover:bg-gray-200 hover:text-gray-600'
              title='Clear filters'
            >
              <XIcon className='h-4 w-4' />
            </button>
          )}
        </div>
        {/* <IndexSearchInput query={query} setQuery={setQuery} isSearchLoading={isSearchLoading} /> */}
        <LabelsButton />
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

const LabelsButton = () => {
  const { scope } = useScope()

  return (
    <Link href={`/${scope}/labels`}>
      <Button variant='primary' size={'base'}>
        Labels
      </Button>
    </Link>
  )
}
