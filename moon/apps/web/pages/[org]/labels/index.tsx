import React, { AwaitedReactNode, JSX, ReactElement, ReactNode, ReactPortal, useCallback, useEffect, useState } from 'react'
import { colord } from 'colord'
import { useRouter } from 'next/router'
import { useDebounce } from 'use-debounce'

import { PostApiLabelListData } from '@gitmono/types'
import {Button, cn, LazyLoadingSpinner, SearchIcon} from '@gitmono/ui'

import { IndexPageContainer, IndexPageContent } from '@/components/IndexPages/components'
import { IssueList as LabelList, ListItem } from '@/components/Issues/IssueList'
import { Pagination } from '@/components/Issues/Pagenation'
import { labelsOpenCurrentPage } from '@/components/Issues/utils/store'
import { AppLayout } from '@/components/Layout/AppLayout'
import { Heading } from '@/components/MrView/catalyst/heading'
import AuthAppProviders from '@/components/Providers/AuthAppProviders'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'
import { usePostLabelList } from '@/hooks/usePostLabelList'
import { apiErrorToast } from '@/utils/apiErrorToast'

type ItemsType = NonNullable<PostApiLabelListData['data']>['items']

function LabelsPage() {
  const router = useRouter()
  const { scope } = useScope()

  const [query, setQuery] = useState("")
  const [queryDebounced] = useDebounce(query, 150)
  const [isLoading, setIsLoading] = useState(false)
  const [isSearchLoading, setIsSearchLoading] = useState(false)

  const handleQuery = () => {
    setIsSearchLoading(true)
    setShowLabelList(
      () =>
        labelList.filter((i) =>
          i.name.toLowerCase().includes(queryDebounced.toLowerCase())))
    setIsSearchLoading(false)
  }

  const [labelList, setLabelList] = useState<ItemsType>([])
  const [showLabelList, setShowLabelList] = useState<ItemsType>([])
  const [numTotal, setNumTotal] = useState(0)
  const [page, setPage] = useState(1)
  const [per_page] = useState(20)
  const { mutate: fetchLabelList } = usePostLabelList()
  const fetchLabels = useCallback(() => {
    setIsLoading(true)

    fetchLabelList(
      {
        data: {
          additional: 'string',
          pagination: {
            page,
            per_page
          }
        }
      },
      {
        onSuccess: (response) => {
          const data = response.data

          setLabelList(data?.items ?? [])
          setShowLabelList(data?.items ?? [])
          setNumTotal(data?.total ?? 0)
        },
        onError: apiErrorToast,
        onSettled: () => setIsLoading(false)
      }
    )
  }, [page, per_page, fetchLabelList])

  useEffect(() => {
    fetchLabels()
  }, [fetchLabels])

  return (
    <>
      <div className='m-4'>
        <Heading>Labels</Heading>
        <br />

        <IndexPageContainer>
          <IndexPageContent id='/[org]/labels' className={cn('@container', '3xl:max-w-6xl max-w-6xl')}>
            <BreadcrumbTitlebar className='justify-between pl-3 pr-3'>
              <div className='relative flex flex-1 flex-row items-center gap-2 overflow-hidden rounded-md border border-gray-300 px-2 py-1 focus-within:border-blue-500 focus-within:ring-1 focus-within:ring-blue-500'>
                <input
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
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault()
                      e.stopPropagation()
                      handleQuery()
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
            </BreadcrumbTitlebar>
            <LabelList
              isLoading={isLoading}
              Issuelists={showLabelList}
              header={
                <BreadcrumbTitlebar className='justify-between bg-gray-100 pl-3 pr-3'>
                  <span className='p-2 font-medium'>{numTotal} labels</span>
                </BreadcrumbTitlebar>
              }
            >
              {(labels) => {
                return labels.map((label) => {
                  const fontColor = colord(label.color).lighten(0.5).toHex()

                  return (
                    <ListItem
                      key={label.id}
                      title={''}
                      onClick={() => router.push(`/${scope}/issue?q=label:${label.name}`)}
                    >
                      <div className='flex items-center gap-2'>
                        <div
                          style={{
                            backgroundColor: label.color,
                            color: fontColor,
                            border: `1px solid ${fontColor}`,
                            borderRadius: '16px',
                            padding: '2px 8px',
                            fontSize: '12px',
                            fontWeight: '500',
                            justifyContent: 'center',
                            textAlign: 'center'
                          }}
                        >
                          {label.name}
                        </div>
                        <div className='flex-1 text-center'>
                          <span className='text-gray-500'>description: {label.description}</span>
                        </div>
                      </div>
                    </ListItem>
                  )
                })
              }}
            </LabelList>
            {numTotal > per_page && (
              <Pagination
                totalNum={numTotal}
                pageSize={per_page}
                onChange={setPage}
                currentPage={labelsOpenCurrentPage}
              />
            )}
          </IndexPageContent>
        </IndexPageContainer>
      </div>
    </>
  )
}

LabelsPage.getProviders = (
  page:
    | string
    | number
    | boolean
    | ReactElement
    | Iterable<ReactNode>
    | ReactPortal
    | Promise<AwaitedReactNode>
    | null
    | undefined,
  pageProps: JSX.IntrinsicAttributes & { children?: ReactNode }
) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default LabelsPage
