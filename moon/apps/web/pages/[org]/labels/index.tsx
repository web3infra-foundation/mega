import React, { AwaitedReactNode, JSX, ReactElement, ReactNode, ReactPortal, useCallback, useEffect, useState } from 'react';
import { Colord, colord } from 'colord';
import { useRouter } from 'next/router';
import { useDebounce } from 'use-debounce';
import { LabelItem } from '@gitmono/types'
import { Button, cn, LazyLoadingSpinner, SearchIcon } from '@gitmono/ui';
import { IndexPageContainer, IndexPageContent } from '@/components/IndexPages/components';
import { IssueList as LabelList, ListItem } from '@/components/Issues/IssueList';
import { Pagination } from '@/components/Issues/Pagenation';
import { labelsOpenCurrentPage } from '@/components/Issues/utils/store';
import { AppLayout } from '@/components/Layout/AppLayout';
import { Heading } from '@/components/MrView/catalyst/heading';
import AuthAppProviders from '@/components/Providers/AuthAppProviders';
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar';
import { useScope } from '@/contexts/scope';
import { usePostLabelList } from '@/hooks/usePostLabelList';
import { apiErrorToast } from '@/utils/apiErrorToast';
import { NewLabelDialog } from '@/components/Labels/NewLabelDialog'
import { usePostLabelNew } from '@/hooks/usePostLabelNew'
import { atomFamily } from 'jotai/utils'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'
import { useAtom } from 'jotai'

const labelListAtom = atomFamily((scope: string) =>
  atomWithWebStorage<LabelItem[]>(`${scope}:issue-label`, [])
)

function LabelsPage() {
  const router = useRouter()
  const { scope } = useScope()

  const [query, setQuery] = useState("")
  const [queryDebounced] = useDebounce(query, 150)
  const [isLoading, setIsLoading] = useState(false)
  const [isSearchLoading, setIsSearchLoading] = useState(false)
  
  const [labelList, setLabelList] = useAtom(labelListAtom(`${scope}`))
  const [numTotal, setNumTotal] = useState(0)
  const [page, setPage] = useState(1)
  const [per_page] = useState(20)
  const { mutate: fetchLabelList } = usePostLabelList()
  const { mutate: postNewLabel } = usePostLabelNew()
  const fetchLabels = useCallback(() => {
    setIsLoading(true)
    setIsSearchLoading(true)
    
    fetchLabelList(
      {
        data: {
          additional: queryDebounced,
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
          setNumTotal(data?.total ?? 0)
        },
        onError: apiErrorToast,
        onSettled: () => {
          setIsLoading(false)
          setIsSearchLoading(false)
        }
      }
    )
  }, [fetchLabelList, queryDebounced, page, per_page, setLabelList])

  useEffect(() => {
    fetchLabels()
  }, [fetchLabels])

  const [isNewLabelDialogOpen, setIsNewLabelDialogOpen] = useState(false);
  const handleCreateLabel = (name: string, description: string, color: string) => {
    postNewLabel(
      {
        data: {
          name,
          description,
          color
        }
      }
    )

    setIsNewLabelDialogOpen(false);

    fetchLabelList(
      {
        data: {
          additional: "",
          pagination: {
            page: 1,
            per_page
          }
        }
      },
      {
        onSuccess: (response) => {
          const data = response.data

          setLabelList(data?.items ?? [])
          setNumTotal(data?.total ?? 0)
          setQuery("")
        },
        onError: apiErrorToast
      }
    )
  };

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
                />
                <span className='text-tertiary flex h-5 w-5 items-center justify-center'>
                  <div className='border-l !border-l-[#d1d9e0]'>
                    <Button variant='plain' className='rounded-none bg-[#f6f8fa]' tooltip='search'>
                      {isSearchLoading ? <LazyLoadingSpinner fallback={<SearchIcon />} /> : <SearchIcon />}
                    </Button>
                  </div>
                </span>
              </div>
              <Button
                variant='primary'
                className='bg-[#1f883d]'
                size={'base'}
                onClick={() => setIsNewLabelDialogOpen(true)}
              >
                New Label
              </Button>
            </BreadcrumbTitlebar>
            <LabelList
              isLoading={isLoading}
              Issuelists={labelList}
              header={
                <BreadcrumbTitlebar className='justify-between bg-gray-100 pl-3 pr-3'>
                  <span className='p-2 font-medium'>{numTotal} labels</span>
                </BreadcrumbTitlebar>
              }
            >
              {(labels) => {
                return labels.map((label) => {
                  const isDark = colord(label.color).isDark()
                  let fontColor: Colord | string = colord(label.color)

                  if(isDark) fontColor = fontColor.lighten(0.4).toHex()
                  else fontColor = fontColor.darken(0.5).toHex()

                  return (
                    <ListItem
                      key={label.id}
                      title={''}
                      onClick={() => router.push(`/${scope}/issue?q=label:${label.name}`)}
                      rightIcon={
                        <div className='self-auto text-center text-gray-500 text-sm'>
                          {label.description}
                        </div>
                      }
                    >
                        <div
                          style={{
                            backgroundColor: label.color,
                            color: fontColor,
                            border: `1px solid ${fontColor}`,
                            borderRadius: '16px',
                            padding: '2px 8px',
                            fontSize: '12px',
                            fontWeight: '700',
                            justifyContent: 'center',
                            textAlign: 'center'
                          }}
                        >
                          {label.name}
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

      <NewLabelDialog
        isOpen={isNewLabelDialogOpen}
        onClose={() => setIsNewLabelDialogOpen(false)}
        onCreateLabel={handleCreateLabel}
      />
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
