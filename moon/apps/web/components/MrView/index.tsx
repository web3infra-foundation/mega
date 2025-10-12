'use client'

import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { GitMergeIcon, GitPullRequestClosedIcon, GitPullRequestIcon } from '@primer/octicons-react'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'
import { useRouter } from 'next/router'

import { LabelItem, SyncOrganizationMember as Member, PostApiMrListData } from '@gitmono/types/generated'
import { Button, CheckIcon, ChevronDownIcon, OrderedListIcon } from '@gitmono/ui'
import { cn } from '@gitmono/ui/src/utils'

import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'
import { IssueIndexTabFilter as MRIndexTabFilter } from '@/components/Issues/IssueIndex'
import {
  Dropdown,
  DropdownItemwithAvatar,
  DropdownItemwithLabel,
  DropdownOrder,
  DropdownReview,
  ListBanner,
  ListItem as MrItem,
  IssueList as MrList
} from '@/components/Issues/IssueList'
import { useScope } from '@/contexts/scope'
import { useGetLabelList } from '@/hooks/useGetLabelList'
import { usePostMrList } from '@/hooks/usePostMrList'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

import { IndexPageContainer, IndexPageContent } from '../IndexPages/components'
import { AdditionType, ItemLabels, ItemRightIcons } from '../Issues/IssuesContent'
import { Pagination } from '../Issues/Pagenation'
import { orderTags, reviewTags } from '../Issues/utils/consts'
import { generateAllMenuItems, MenuConfig } from '../Issues/utils/generateAllMenuItems'
import { filterAtom, mrCloseCurrentPage, mridAtom, mrOpenCurrentPage, sortAtom } from '../Issues/utils/store'
import { Heading } from './catalyst/heading'

// interface MrInfoItem {
//   link: string
//   title: string
//   status: string
//   open_timestamp: number
//   merge_timestamp: number | null
//   updated_at: number
// }

type ItemsType = NonNullable<PostApiMrListData['data']>['items']

export default function MrView() {
  const { scope } = useScope()
  const [mrList, setMrList] = useState<ItemsType>([])
  const [numTotal, setNumTotal] = useState(0)
  const [pageSize] = useState(10)
  const [status, _setStatus] = useAtom(filterAtom({ part: 'mr' }))
  // const [status, _setStatus] = useState('open')
  const [page, setPage] = useState(1)
  const [isLoading, setIsLoading] = useState(false)
  const { mutate: fetchMrList } = usePostMrList()
  const [sort, setSort] = useAtom(sortAtom({ scope, filter: 'sortPickerMR' }))
  const { members } = useSyncedMembers()
  const { labels: labelList } = useGetLabelList()
  const [_mrid, setMrid] = useAtom(mridAtom)

  const orderAtom = useMemo(
    () => atomWithWebStorage(`${scope}:mr-order`, { sort: 'Created On', time: 'Newest' }),
    [scope]
  )

  // const [openCurrent, setopenCurrent] = useAtom(mrOpenCurrentPage)
  // const [closeCurrent, setcloseCurrent] = useAtom(mrCloseCurrentPage)

  const reviewAtom = useMemo(() => atomWithWebStorage(`${scope}:mr-review`, ''), [scope])

  const labelAtom = useMemo(() => atomWithWebStorage<string[]>(`${scope}:mr-label`, []), [scope])

  const [order, setOrder] = useAtom(orderAtom)

  const [label, setLabel] = useAtom(labelAtom)

  const [review, setReview] = useAtom(reviewAtom)

  const additions = useCallback(
    (labels: number[]): AdditionType => {
      const additional: AdditionType = { status, asc: false }

      if (sort['Assignees']) additional.assignees = [sort['Assignees']]

      if (sort['Author']) additional.author = sort['Author'] as string

      if (labels.length) additional.labels = [...labels]

      if (order.time === 'Newest') {
        additional.asc = false
        additional.sort_by = handleSort(order['sort'])
      } else if (order.time === 'Oldest') {
        additional.asc = true
        additional.sort_by = handleSort(order['sort'])
      }
      return additional
    },
    [order, sort, status]
  )

  const loadMrList = useCallback(
    (additional?: AdditionType) => {
      setIsLoading(true)
      const addittion = additional ? additional : additions([])

      fetchMrList(
        {
          data: {
            pagination: {
              page,
              per_page: pageSize
            },
            additional: addittion
          }
        },
        {
          onSuccess: (response) => {
            const data = response.data

            // setMrList(
            //   data?.items?.map((item) => ({
            //     ...item,
            //     merge_timestamp: item.merge_timestamp ?? null
            //   })) ?? []
            // )
            setMrList(data?.items ?? [])
            setNumTotal(data?.total ?? 0)
          },
          onError: apiErrorToast,
          onSettled: () => setIsLoading(false)
        }
      )
    },
    [page, pageSize, fetchMrList, additions]
  )

  useEffect(() => {
    loadMrList()
  }, [loadMrList])

  const handleSort = (str: string): string => {
    switch (str) {
      case 'Created on':
        return 'created_at'
      case 'Last updated':
        return 'updated_at'

      default:
        return 'Created on'
    }
  }

  const getStatusIcon = (status: string) => {
    const normalizedStatus = status.toLowerCase()

    switch (normalizedStatus) {
      case 'open':
        return <GitPullRequestIcon className='text-[#378f50]' />
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
              <span className='cursor-pointer hover:text-blue-600 hover:underline'>
                {item.author}
              </span>
            </MemberHovercard>

          </>
        )
      case 'merged':
        if (item.merge_timestamp !== null) {
          return (
            <>
              by{' '}
              <MemberHovercard username={item.author}>
                <span className='cursor-pointer hover:text-blue-600 hover:underline'>
                  {item.author}
                </span>
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
              <span className='cursor-pointer hover:text-blue-600 hover:underline'>
                {item.author}
              </span>
            </MemberHovercard>

            {' was closed '}

            {formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}


          </>
        )
      default:
        return null
    }
  }

  const MemberConfig: MenuConfig<Member>[] = [
    {
      key: 'Author',
      isChosen: (item) => item.user.username === sort['Author'],
      onSelectFactory: (item: Member) => (e: Event) => {
        e.preventDefault()
        if (item.user.username === sort['Author']) {
          loadMrList()
          setSort({
            ...sort,
            Author: ''
          })
        } else {
          setSort({
            ...sort,
            Author: item.user.username
          })
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item: Member) => <DropdownItemwithAvatar member={item} classname='text-sm' />
    },
    {
      key: 'Assignees',
      isChosen: (item: Member) => item.user.username === sort['Assignees'],
      onSelectFactory: (item: Member) => (e: Event) => {
        e.preventDefault()
        if (item.user.username === sort['Assignees']) {
          loadMrList()
          setSort({
            ...sort,
            Assignees: ''
          })
        } else {
          setSort({
            ...sort,
            Assignees: item.user.username
          })
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item: Member) => <DropdownItemwithAvatar member={item} classname='text-sm' />
    }
  ]

  const LabelConfig: MenuConfig<LabelItem>[] = [
    {
      key: 'Labels',
      isChosen: (item) => label?.includes(String(item.id)),

      onSelectFactory: (item) => (e: Event) => {
        e.preventDefault()
        if (label?.includes(String(item.id))) {
          setLabel(label.filter((i) => i !== String(item.id)))
        } else {
          setLabel([...label, String(item.id)])
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item) => <DropdownItemwithLabel label={item} />
    }
  ]

  const ReviewConfig: MenuConfig<string>[] = [
    {
      key: 'Review',
      isChosen: () => true,

      onSelectFactory: (item) => (e: Event) => {
        e.preventDefault()
        if (item === review) {
          setReview('')
        } else {
          setReview(item)
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item) => (
        <div className='flex items-center gap-2'>
          <div className='h-4 w-4'>{review === item && <CheckIcon />}</div>
          <span className='flex-1'>{item}</span>
        </div>
      )
    }
  ]

  const OrderConfig: MenuConfig<string>[] = [
    {
      key: 'Order',
      isChosen: (item) => item === 'Newest' || item === 'Oldest',

      onSelectFactory: (item) => (e: Event) => {
        e.preventDefault()
        if (item === 'Newest') {
          setOrder({
            ...order,
            time: 'Newest'
          })
        } else if (item === 'Oldest') {
          setOrder({
            ...order,
            time: 'Oldest'
          })
        } else {
          setOrder({
            ...order,
            sort: item
          })
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item) => (
        <div className='flex items-center gap-2'>
          <div className='h-4 w-4'>
            {order.sort === item && <CheckIcon />}
            {order.time === item && <CheckIcon />}
          </div>
          <span className='flex-1'>{item}</span>
        </div>
      )
    }
  ]

  const handleOpen = (open: boolean) => {
    if (!open) {
      const news = label.map((i) => Number(i))
      const addtion = additions(news)

      loadMrList(addtion)
    }
  }

  const member = generateAllMenuItems(members, MemberConfig)

  const labels = generateAllMenuItems(labelList, LabelConfig)

  const orders = generateAllMenuItems(orderTags, OrderConfig)

  const reviews = generateAllMenuItems(reviewTags, ReviewConfig)

  const ListHeaderItem = (p: string) => {
    switch (p) {
      case 'Author':
        return (
          <Dropdown
            isChosen={sort['Author'] === ''}
            key={p}
            name={p}
            dropdownArr={member?.get('Author').all}
            dropdownItem={member?.get('Author').chosen}
          />
        )
      case 'Assignees':
        return (
          <Dropdown
            isChosen={sort['Assignees'] === ''}
            key={p}
            name={p}
            dropdownArr={member?.get('Assignees').all}
            dropdownItem={member?.get('Assignees').chosen}
          />
        )
      case 'Reviews':
        return (
          <DropdownReview
            key={p}
            name={p}
            dropdownArr={reviews?.get('Review').all}
            dropdownItem={reviews?.get('Review').chosen}
          />
        )
      case 'Labels':
        return (
          <Dropdown
            onOpen={handleOpen}
            isChosen={!label?.length}
            key={p}
            name={p}
            dropdownArr={labels?.get('Labels').all}
            dropdownItem={labels?.get('Labels').chosen}
          />
        )
      case `${order.sort}`:
        return (
          <DropdownOrder
            key={p}
            name={p}
            dropdownArr={orders?.get('Order').all}
            dropdownItem={orders?.get('Order').chosen}
            inside={
              <>
                <div className='flex items-center'>
                  {p}
                  <OrderedListIcon />
                </div>
              </>
            }
          />
        )
      default:
        return (
          <>
            <Button size='sm' variant={'plain'} tooltipShortcut={p}>
              <div className='flex items-center justify-center'>
                {p}
                <ChevronDownIcon />
              </div>
            </Button>
          </>
        )
    }
  }

  const handlePageChange = (page: number) => {
    setPage(page)
  }

  const router = useRouter()

  return (
    <div className='relative m-4 flex h-screen flex-col'>
      <Heading>Merge Request</Heading>
      <br />
      <IndexPageContainer>
        <IndexPageContent id='/[org]/mr' className={cn('@container', '3xl:max-w-7xl max-w-7xl')}>
          <div className='flex h-full flex-col'>
            <MrList
              isLoading={isLoading}
              Issuelists={mrList}
              header={
                <ListBanner
                  pickerTypes={[
                    'Author',
                    'Labels',
                    'Projects',
                    'Milestones',
                    'Reviews',
                    'Assignees',
                    'Types',
                    `${order.sort}`
                  ]}
                  tabfilter={<MRIndexTabFilter part='mr' />}
                >
                  {(p) => ListHeaderItem(p)}
                </ListBanner>
              }
            >
              {(issueList) => {
                return issueList.map((i) => (
                  <MrItem
                    key={i.id}
                    title={i.title}
                    leftIcon={getStatusIcon(i.status)}
                    labels = {<ItemLabels item={i} />}
                    rightIcon={<ItemRightIcons item={i} />}
                    onClick={() => {
                      setMrid(i.id)
                      router.push(`/${scope}/mr/${i.link}`)
                    }}
                  >
                    <div className='text-xs text-[#59636e]'>
                      #{i.link}{'    '}
                      {getDescription(i)}
                      {' • MergeRequest'}
                    </div>
                  </MrItem>
                ))
              }}
            </MrList>
            <div className='mt-auto'>
              {status === 'open' && (
                <div className='mt-auto'>
                  <Pagination
                    totalNum={numTotal}
                    currentPage={mrOpenCurrentPage}
                    pageSize={pageSize}
                    onChange={(page: number) => handlePageChange(page)}
                  />
                </div>
              )}
              {status === 'closed' && (
                <div className='mt-auto'>
                  <Pagination
                    totalNum={numTotal}
                    currentPage={mrCloseCurrentPage}
                    pageSize={pageSize}
                    onChange={(page: number) => handlePageChange(page)}
                  />
                </div>
              )}
            </div>
          </div>
        </IndexPageContent>
      </IndexPageContainer>
    </div>
  )
}
