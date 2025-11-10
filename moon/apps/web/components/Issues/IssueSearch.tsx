// import { useState } from 'react'
// import { useDebounce } from 'use-debounce'

import { XIcon } from '@primer/octicons-react'

import { Button, Link, SearchIcon } from '@gitmono/ui'

import { useScope } from '@/contexts/scope'
import React from 'react'

// import Search from './Search'
// import { fuseOptions, searchList } from './utils/consts'

interface IssueSearchProps {
  filterQuery?: string
  onClearFilters?: () => void
}

export default function IssueSearch({ filterQuery, onClearFilters }: IssueSearchProps) {

  return (
    <>
      <div className='flex items-center gap-2 min-h-[35px]'>

        <div className='flex-1 group flex min-h-[35px] items-center rounded-md border border-gray-300 bg-white px-3  shadow-sm transition-all focus-within:border-blue-500 focus-within:shadow-md focus-within:ring-2 focus-within:ring-blue-100 hover:border-gray-400'>
          <div className='flex items-center text-gray-400'>
            <SearchIcon className=' w-4' />
          </div>

          <input
            type='text'
            value={filterQuery || ''}
            readOnly
            placeholder='Filter issues by author, labels, or assignee...'
            className='w-full flex-1 border-none bg-transparent  text-sm text-gray-400 outline-none ring-0 focus:outline-none focus:ring-0'
          />
          {filterQuery && (
            <button
              onClick={onClearFilters}
              className='flex items-center justify-center rounded-md p-1 text-gray-400 transition-all hover:bg-gray-200 hover:text-gray-600'
              title='Clear filters'
            >
              <XIcon className='h-4 w-4' />
            </button>
          )}
        </div>

        <LabelsButton />
        <NewIssueButton />
      </div>

    </>
  )
}

const LabelsButton = () => {
  const { scope } = useScope()

  return (
    <Link href={`/${scope}/labels`}>
      <Button variant='primary' size={'base'} >
        Labels
      </Button>
    </Link>
  )
}

export const NewIssueButton = () => {
  const { scope } = useScope()

  return (
    <Link href={`/${scope}/issue/new`}>
      <Button variant='primary' className='bg-[#1f883d] ' size={'base'}>
        New Issue
      </Button>
    </Link>
  )
}
