'use client'

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  GitMergeIcon,
  GitPullRequestClosedIcon,
  GitPullRequestDraftIcon,
  GitPullRequestIcon,
  XIcon
} from '@primer/octicons-react'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'
import { useRouter } from 'next/router'

import { PostApiClListData } from '@gitmono/types/generated'
import { SearchIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import {
  List as ClList,
  ListBanner as ClListBanner,
  ListItem as ClListItem,
  IndexTabFilter,
  ItemLabels,
  ItemRightIcons
} from '@/components/ClView/ClList'
import {
  AssigneesDropdown,
  AuthorDropdown,
  LabelsDropdown,
  MilestonesDropdown,
  OrderDropdown,
  ProjectsDropdown,
  ReviewDropdown,
  TypesDropdown,
  useFilterState
} from '@/components/ClView/filters'
import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'
import { IssueBreadcrumbIcon } from '@/components/Titlebar/BreadcrumbPageIcons'
import { BreadcrumbTitlebar } from '@/components/Titlebar/BreadcrumbTitlebar'
import { useScope } from '@/contexts/scope'
import { usePostClList } from '@/hooks/CL/usePostClList'
import { useGetLabelList } from '@/hooks/useGetLabelList'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

import { IndexPageContainer, IndexPageContent } from '../IndexPages/components'
import { Pagination } from '../Issues/Pagenation'
import { clIdAtom } from '../Issues/utils/store'
import { useDraftClList } from './hook/useDraftClList'

type ItemsType = NonNullable<PostApiClListData['data']>['items']

export default function CLView() {
  const { scope } = useScope()
  const router = useRouter()

  const [clList, setClList] = useState<ItemsType>([])
  const [numTotal, setNumTotal] = useState(0)
  const [pageSize] = useState(10)
  const [page, setPage] = useState(1)
  const [isLoading, setIsLoading] = useState(false)
  const [status, setStatus] = useState('open')

  const [_clid, setClid] = useAtom(clIdAtom)

  const { mutate: fetchClList } = usePostClList()
  const { mutate: fetchDraftClList } = useDraftClList()
  const { members } = useSyncedMembers()

  const filterState = useFilterState({ scope: scope as string, type: 'cl' })
  const filterStateRef = useRef(filterState)

  const { labels } = useGetLabelList()

  const orderAtom = useMemo(
    () => atomWithWebStorage(`${scope}:cl-order`, { sort: 'Created On', time: 'Newest' }),
    [scope]
  )
  const [order, setOrder] = useAtom(orderAtom)
  const orderRef = useRef(order)

  useEffect(() => {
    orderRef.current = order
    filterStateRef.current = filterState
  }, [order, filterState])

  const searchQuery = useMemo(() => {
    return filterState.toQueryString(labels)
  }, [filterState, labels])

  const clearAllFilters = () => {
    filterState.clearAllFilters()
    if (page !== 1) {
      setPage(1)
    } else {
      setTimeout(() => fetchClListData(), 0)
    }
  }

  const fetchClListData = useCallback(() => {
    setIsLoading(true)

    const params = filterStateRef.current.toApiParams()
    const currentOrder = orderRef.current
    const baseAdditional: any = {
      sort_by: handleSort(currentOrder.sort),
      ...params
    }

    if (status === 'draft') {
      fetchDraftClList(
        {
          data: {
            pagination: { page, per_page: pageSize },
            additional: baseAdditional
          }
        },
        {
          onSuccess: (response) => {
            const data = response.data

            setClList((data?.items ?? []) as ItemsType)
            setNumTotal(data?.total ?? 0)
          },
          onError: apiErrorToast,
          onSettled: () => setIsLoading(false)
        }
      )
    } else if (status === 'open') {
      const additional: any = {
        ...baseAdditional,
        status: 'open',
        asc: currentOrder.time === 'Oldest'
      }

      let openData: PostApiClListData['data'] | undefined
      let draftData: PostApiClListData['data'] | undefined
      let openFinished = false
      let draftFinished = false

      const finalize = () => {
        if (!openFinished || !draftFinished) return

        const openItems = (openData?.items ?? []) as ItemsType
        const draftItems = (draftData?.items ?? []) as ItemsType

        setClList([...openItems, ...draftItems])
        setNumTotal((openData?.total ?? 0) + (draftData?.total ?? 0))
        setIsLoading(false)
      }

      fetchClList(
        {
          data: {
            pagination: { page, per_page: pageSize },
            additional
          }
        },
        {
          onSuccess: (response) => {
            openData = response.data
          },
          onError: apiErrorToast,
          onSettled: () => {
            openFinished = true
            finalize()
          }
        }
      )

      fetchDraftClList(
        {
          data: {
            pagination: { page, per_page: pageSize },
            additional: baseAdditional
          }
        },
        {
          onSuccess: (response) => {
            draftData = response.data
          },
          onError: apiErrorToast,
          onSettled: () => {
            draftFinished = true
            finalize()
          }
        }
      )
    } else {
      const additional: any = {
        ...baseAdditional,
        status,
        asc: currentOrder.time === 'Oldest'
      }

      fetchClList(
        {
          data: {
            pagination: { page, per_page: pageSize },
            additional
          }
        },
        {
          onSuccess: (response) => {
            const data = response.data

            setClList((data?.items ?? []) as ItemsType)
            setNumTotal(data?.total ?? 0)
          },
          onError: apiErrorToast,
          onSettled: () => setIsLoading(false)
        }
      )
    }
  }, [page, pageSize, status, fetchClList, fetchDraftClList])

  useEffect(() => {
    fetchClListData()
  }, [page, pageSize, status, fetchClListData])

  const handleSort = (str: string): string => {
    switch (str) {
      case 'Created on':
        return 'created_at'
      case 'Last updated':
        return 'updated_at'

      default:
        return 'created_at'
    }
  }

  const handleFilterClose = useCallback(() => {
    if (!filterState.hasChanged()) {
      return
    }
    const currentFilterString = filterState.toQueryString(labels)

    if (!currentFilterString || currentFilterString.trim() === '') {
      return
    }

    if (page !== 1) {
      setPage(1)
    } else {
      fetchClListData()
    }
  }, [filterState, labels, page, fetchClListData])

  const handleOrderChange = useCallback(
    (sort: string, time: string) => {
      if (order.sort === sort && order.time === time) {
        return
      }

      setOrder({ sort, time })
      if (page !== 1) {
        setPage(1)
      } else {
        setTimeout(() => fetchClListData(), 0)
      }
    },
    [order.sort, order.time, setOrder, page, fetchClListData]
  )

  const getStatusIcon = (status: string) => {
    const normalizedStatus = status.toLowerCase()

    switch (normalizedStatus) {
      case 'open':
        return <GitPullRequestIcon className='text-[#378f50]' />
      case 'draft':
        return <GitPullRequestDraftIcon className='text-[#6e7781]' />
      case 'closed':
        return <GitPullRequestClosedIcon className='text-[#d1242f]' />
      case 'merged':
        return <GitMergeIcon className='text-[#8250df]' />
      default:
        return null
    }
  }

  const getDescription = (item: ItemsType[number]) => {
    const normalizedStatus = item.status.toLowerCase()

    switch (normalizedStatus) {
      case 'open':
        return (
          <>
            opened {formatDistance(fromUnixTime(item.open_timestamp), new Date(), { addSuffix: true })} by{' '}
            <MemberHovercard username={item.author}>
              <span className='cursor-pointer hover:text-blue-600 hover:underline'>{item.author}</span>
            </MemberHovercard>
          </>
        )
      case 'merged':
        if (item.merge_timestamp !== null) {
          return (
            <>
              by{' '}
              <MemberHovercard username={item.author}>
                <span className='cursor-pointer hover:text-blue-600 hover:underline'>{item.author}</span>
              </MemberHovercard>
              {' was merged '}
              {formatDistance(fromUnixTime(item.merge_timestamp ?? 0), new Date(), { addSuffix: true })}
            </>
          )
        } else {
          return ''
        }
      case 'closed':
        return (
          <>
            by{' '}
            <MemberHovercard username={item.author}>
              <span className='cursor-pointer hover:text-blue-600 hover:underline'>{item.author}</span>
            </MemberHovercard>
            {' was closed '}
            {formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}
          </>
        )
      default:
        return null
    }
  }

  return (
    <>
      <IndexPageContainer>
        <BreadcrumbTitlebar>
          <IssueBreadcrumbIcon />
        </BreadcrumbTitlebar>

        <IndexPageContent
          id='/[org]/cl'
          className={cn('@container', 'max-w-full lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl')}
        >
          <div className='group flex min-h-[35px] items-center rounded-md border border-gray-300 bg-white px-3 shadow-sm transition-all focus-within:border-blue-500 focus-within:shadow-md focus-within:ring-2 focus-within:ring-blue-100 hover:border-gray-400'>
            <div className='flex items-center text-gray-400'>
              <SearchIcon className='h-4 w-4' />
            </div>

            <input
              type='text'
              value={searchQuery}
              readOnly
              placeholder='Filter change list by author, labels , assignee, or review...'
              className='w-full flex-1 border-none bg-transparent text-sm text-gray-400 outline-none ring-0 focus:outline-none focus:ring-0'
            />

            {searchQuery && (
              <button
                onClick={() => {
                  clearAllFilters()
                }}
                className='flex items-center justify-center rounded-md p-1 text-gray-400 transition-all hover:bg-gray-100 hover:text-gray-600'
                title='Clear search'
              >
                <XIcon className='h-4 w-4' />
              </button>
            )}
          </div>

          <ClList
            isLoading={isLoading}
            lists={clList}
            header={
              <ClListBanner
                tabfilter={
                  <IndexTabFilter
                    part={status}
                    setPart={(newStatus) => {
                      setStatus(newStatus)
                      setPage(1)
                    }}
                  />
                }
              >
                {/* Author, Labels, Projects, Milestones, Reviews, Assignees, Types, Order */}
                <AuthorDropdown
                  members={members}
                  value={filterState.filters.author}
                  onChange={filterState.setAuthor}
                  onClose={handleFilterClose}
                />
                <LabelsDropdown
                  labels={labels}
                  value={filterState.filters.labels}
                  onChange={filterState.setLabels}
                  onClose={handleFilterClose}
                />
                <ProjectsDropdown />
                <MilestonesDropdown />
                <ReviewDropdown
                  options={['Approved', 'Changes requested', 'Commented', 'Pending']}
                  value={filterState.filters.review || ''}
                  onChange={filterState.setReview}
                  onClose={handleFilterClose}
                />
                <AssigneesDropdown
                  members={members}
                  value={filterState.filters.assignees}
                  onChange={filterState.setAssignees}
                  onClose={handleFilterClose}
                />
                <TypesDropdown />
                <OrderDropdown
                  sortOptions={['Created On', 'Last updated']}
                  timeOptions={['Newest', 'Oldest']}
                  currentSort={order.sort}
                  currentTime={order.time}
                  onChange={handleOrderChange}
                />
              </ClListBanner>
            }
          >
            {(issueList) => {
              return issueList.map((i) => (
                <ClListItem
                  key={i.id}
                  title={i.title}
                  leftIcon={getStatusIcon(i.status)}
                  labels={<ItemLabels item={i} />}
                  rightIcon={<ItemRightIcons item={i} />}
                  onClick={() => {
                    setClid(i.id)
                    router.push(`/${scope}/cl/${i.link}`)
                  }}
                >
                  <div className='text-xs text-[#59636e]'>
                    <span className='mr-2'>#{i.link}</span>
                    {getDescription(i)}
                    {' • ChangeList'}
                  </div>
                </ClListItem>
              ))
            }}
          </ClList>
          <Pagination
            totalNum={numTotal}
            currentPage={page}
            pageSize={pageSize}
            onChange={(page: number) => setPage(page)}
          />

          {/*</div>*/}
        </IndexPageContent>
      </IndexPageContainer>
    </>
  )
}
