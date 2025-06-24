import React, { useEffect, useMemo, useRef } from 'react'
import Fuse, { IFuseOptions } from 'fuse.js'

import { Button, LayeredHotkeys, LazyLoadingSpinner, SearchIcon } from '@gitmono/ui'

interface IndexSearchInputProps {
  query: string
  setQuery: (query: string) => void
  isSearchLoading: boolean
}

interface SearchType {
  type: 'separator' | 'item'
  label?: string
}
export interface SearchProps {
  SearchListTable: {
    items: { type: 'separator' | 'item'; label?: string }[]
    open?: boolean
    setOpen?: (open: boolean) => void
  }
  SearchQuery: IndexSearchInputProps
  SearchOptions?: {
    searchList: SearchType[]
    fuseOptions: IFuseOptions<SearchType>
  }
}

export default function Search({ SearchListTable, SearchQuery, SearchOptions }: SearchProps) {
  const { items, open = false, setOpen } = SearchListTable
  const ref = useRef<HTMLInputElement>(null)
  const { query, setQuery, isSearchLoading } = SearchQuery
  const hasNoSearchRes = useRef(false)

  // const fuse = useMemo(() => new Fuse(searchList, fuseOptions), [])
  const fuse = useMemo(
    () => new Fuse(SearchOptions?.searchList ?? [], SearchOptions?.fuseOptions),
    [SearchOptions?.searchList, SearchOptions?.fuseOptions]
  )

  useEffect(() => {
    if (!query) return
    const result = fuse.search(query)

    if (result.length > 0) {
      hasNoSearchRes.current = false
      setOpen?.(true)
    } else if (!hasNoSearchRes.current) {
      hasNoSearchRes.current = true
      setOpen?.(false)
    }
    // SearchListTable.setOpen?.(result.length > 0)
  }, [query, fuse, setOpen])

  const handleNothing = () => {
    setOpen?.(false)
    return [] as SearchProps['SearchListTable']['items']
  }

  return (
    <>
      <div className='relative flex flex-1 flex-row items-center gap-2 overflow-hidden rounded-md border border-gray-300 px-2 py-1 focus-within:border-blue-500 focus-within:ring-1 focus-within:ring-blue-500'>
        <LayeredHotkeys keys='meta+f' callback={() => ref.current?.focus()} options={{ preventDefault: true }} />

        <input
          ref={ref}
          className='flex-1 border-none bg-transparent p-0 text-sm outline-none ring-0 focus:ring-0'
          placeholder='Search...'
          role='searchbox'
          autoComplete='off'
          autoCorrect='off'
          spellCheck={false}
          type='text'
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
          }}
          onFocus={() => setOpen?.(true)}
          onBlur={() => setOpen?.(false)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              setQuery('')
              ref.current?.blur()
            } else if (e.key === 'Enter') {
              e.preventDefault()
              e.stopPropagation()
            }
          }}
        />
        <span className='text-tertiary flex h-5 w-5 items-center justify-center'>
          <div className='border-l !border-l-[#d1d9e0]'>
            <Button variant='plain' className='rounded-none bg-[#f6f8fa]' tooltip='search'>
              {isSearchLoading ? <LazyLoadingSpinner fallback={<SearchIcon />} /> : <SearchIcon />}
            </Button>
          </div>
        </span>
      </div>
      {open && (
        <div className='absolute top-12 z-20 max-h-[50vh] w-[288px] overflow-y-auto rounded-lg border border-gray-200 bg-white px-4 py-2 shadow-lg'>
          {(query
            ? fuse.search(query).length === 0
              ? handleNothing()
              : fuse.search(query).map((i) => i.item)
            : items
          ).map((i, index) => {
            switch (i.type) {
              case 'separator':
                // eslint-disable-next-line react/no-array-index-key
                return <div key={index} className='my-2 border-t border-gray-300'></div>
              case 'item':
                // eslint-disable-next-line react/no-array-index-key
                return <p key={index}>{i.label}</p>
              default:
                // eslint-disable-next-line react/no-array-index-key
                return <p key={index}>test</p>
            }
          })}
        </div>
      )}
    </>
  )
}
