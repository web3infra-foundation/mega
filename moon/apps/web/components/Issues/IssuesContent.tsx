import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { CheckIcon, IssueClosedIcon, IssueOpenedIcon, SkipIcon } from '@primer/octicons-react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'
import { useRouter } from 'next/router'

import {
  LabelItem,
  SyncOrganizationMember as Member,
  PageParamsListPayload,
  PostApiIssueListData
} from '@gitmono/types/generated'
import { Button, ChatBubbleIcon, ChevronDownIcon, OrderedListIcon } from '@gitmono/ui'

// import { MenuItem } from '@gitmono/ui/Menu'

import { EmptySearchResults } from '@/components/Feed/EmptySearchResults'
import {
  Dropdown,
  DropdownItemwithAvatar,
  DropdownItemwithLabel,
  DropdownOrder,
  IssueList,
  ListBanner,
  ListItem
} from '@/components/Issues/IssueList'
import { filterAtom, issueCloseCurrentPage, issueOpenCurrentPage, sortAtom } from '@/components/Issues/utils/store'
import { useScope } from '@/contexts/scope'
import { useGetIssueLists } from '@/hooks/issues/useGetIssueLists'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

import { IssueIndexTabFilter } from './IssueIndex'
import { MemberHoverAvatarList } from './MemberHoverAvatarList'
import { Pagination } from './Pagenation'
import { orderTags } from './utils/consts'
import { generateAllMenuItems, MenuConfig } from './utils/generateAllMenuItems'
import { useGetLabelList } from '@/hooks/useGetLabelList'

interface Props {
  getIssues?: ReturnType<typeof useInfiniteQuery<PostApiIssueListData>>
  searching?: boolean
  hideProject?: boolean
}

export interface Item {
  closed_at?: number | null
  link: string
  author: string
  title: string
  status: string
  open_timestamp: number
  updated_at: number
}

export type ItemsType = NonNullable<PostApiIssueListData['data']>['items']

export type AdditionType = NonNullable<PageParamsListPayload>['additional']

export interface Label {
  id: string
  name: string
  color: string
  remarks: string
  checked: boolean
}

export function IssuesContent({ searching }: Props) {
  const { mutate: issueLists } = useGetIssueLists()

  const { scope } = useScope()

  const [pageSize, _setPageSize] = useState(10)

  const [status, _setStatus] = useAtom(filterAtom({ part: 'issue' }))

  const [issueList, setIssueList] = useState<ItemsType>([])

  const [loading, setLoading] = useState(false)

  const [numTotal, setNumTotal] = useState(0)

  const [sort, setSort] = useAtom(sortAtom({ scope, filter: 'sortPicker' }))

  const orderAtom = useMemo(
    () => atomWithWebStorage(`${scope}:issue-order`, { sort: 'Created On', time: 'Newest' }),
    [scope]
  )
  const labelAtom = useMemo(() => atomWithWebStorage<string[]>(`${scope}:issue-label`, []), [scope])

  const [order, setOrder] = useAtom(orderAtom)

  const [label, setLabel] = useAtom(labelAtom)

  const { members } = useSyncedMembers()
  const { labels } = useGetLabelList()

  const router = useRouter()

  const [openCurrent, setopenCurrent] = useAtom(issueOpenCurrentPage)
  const [closeCurrent, setcloseCurrent] = useAtom(issueCloseCurrentPage)

  const MemberConfig: MenuConfig<Member>[] = [
    {
      key: 'Author',
      isChosen: (item) => item.user.username === sort['Author'],
      onSelectFactory: (item: Member) => (e: Event) => {
        e.preventDefault()
        if (item.user.username === sort['Author']) {
          status === 'open' ? setopenCurrent(1) : setcloseCurrent(1)
          fetchData(1, pageSize)
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
          status === 'open' ? setopenCurrent(1) : setcloseCurrent(1)
          fetchData(1, pageSize)
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

  const additions = useCallback(
    (labels: number[]): AdditionType => {
      const additional: AdditionType = { status, asc: false }

      if (sort['Assignees']) additional.assignees = [sort['Assignees']]

      if (sort['Author']) additional.author = sort['Author'] as string

      if (labels.length) additional.labels = [...labels]

      if (order.time === 'Newest') {
        additional.asc = false
      } else if (order.time === 'Oldest') {
        additional.asc = true
      }
      additional.sort_by = handleSort(order['sort'])
      return additional
    },
    [order, sort, status]
  )

  const member = generateAllMenuItems(members, MemberConfig)

  const labelList = generateAllMenuItems(labels, LabelConfig)

  const orders = generateAllMenuItems(orderTags, OrderConfig)

  const handleOpen = (open: boolean) => {
    if (!open) {
      const news = label.map((i) => Number(i))
      const addtion = additions(news)

      status === 'open' ? setopenCurrent(1) : setcloseCurrent(1)
      fetchData(1, pageSize, addtion)
    }
  }

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
      case 'Labels':
        return (
          <Dropdown
            onOpen={handleOpen}
            isChosen={!label?.length}
            key={p}
            name={p}
            dropdownArr={labelList?.get('Labels').all}
            dropdownItem={labelList?.get('Labels').chosen}
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

  const fetchData = useCallback(
    (page: number, per_page: number, additional?: AdditionType) => {
      setLoading(true)
      const addittion = additional ? additional : additions([])

      issueLists(
        {
          data: {
            pagination: { page, per_page },
            additional: addittion
          }
        },
        {
          onSuccess: (response) => {
            const data = response.data

            setIssueList(data?.items ?? [])
            setNumTotal(data?.total ?? 0)
          },
          onError: apiErrorToast,
          onSettled: () => {
            setLoading(false)
          }
        }
      )
    },

    [issueLists, additions]
  )

  useEffect(() => {
    if (status === 'open') {
      fetchData(openCurrent, pageSize)
    } else if (status === 'closed') {
      fetchData(closeCurrent, pageSize)
    }
  }, [pageSize, fetchData, openCurrent, closeCurrent, status])

  // if (loading) {
  //   return <IndexPageInstantLoading />
  // }

  // if (!issueList.length) {
  //   return searching ? <EmptySearchResults /> : <IssueIndexEmptyState />
  // }
  if (!issueList.length && searching) {
    return <EmptySearchResults />
  }

  const handlePageChange = (page: number) => {
    status === 'open' ? setopenCurrent(page) : setcloseCurrent(page)
  }

  const getStatusIcon = (status: string) => {
    const normalizedStatus = status.toLowerCase()

    switch (normalizedStatus) {
      case 'open':
        return <IssueOpenedIcon className='text-[#378f50]' />
      case 'closed':
        return <IssueClosedIcon className='text-[#8250df]' />
      default:
        return <SkipIcon className='text-[#59636e]' />
    }
  }

  return (
    <>
      {/* TODO:Searching logic need to be completed */}
      {searching ? (
        <>
          <IssueSearchList searchIssueList={issueList} />
          <Pagination currentPage={issueOpenCurrentPage} totalNum={numTotal} pageSize={pageSize} />
        </>
      ) : (
        <>
          <IssueList
            isLoading={loading}
            Issuelists={issueList}
            header={
              <ListBanner
                pickerTypes={['Author', 'Labels', 'Projects', 'Milestones', 'Assignees', 'Types', `${order.sort}`]}
                tabfilter={
                  <IssueIndexTabFilter openTooltip='Issues that are still open and need attention' part='issue' />
                }
              >
                {(p) => ListHeaderItem(p)}
              </ListBanner>
            }
          >
            {(issueList) => {
              return issueList.map((i) => (
                <ListItem
                  key={i.link}
                  title={i.title}
                  leftIcon={getStatusIcon(i.status)}
                  rightIcon={<RightAvatar item={i} />}
                  onClick={() => router.push(`/${scope}/issue/${i.link}/${i.id}`)}
                >
                  <div className='text-xs text-[#59636e]'>
                    {i.link} · {i.author} {i.status}{' '}
                    {formatDistance(fromUnixTime(i.open_timestamp), new Date(), { addSuffix: true })}
                  </div>
                </ListItem>
              ))
            }}
          </IssueList>

          {status === 'open' && (
            <div className='mt-auto'>
              <Pagination
                currentPage={issueOpenCurrentPage}
                totalNum={numTotal}
                pageSize={pageSize}
                onChange={(page: number) => handlePageChange(page)}
              />
            </div>
          )}
          {status === 'closed' && (
            <div className='mt-auto'>
              <Pagination
                currentPage={issueCloseCurrentPage}
                totalNum={numTotal}
                pageSize={pageSize}
                onChange={(page: number) => handlePageChange(page)}
              />
            </div>
          )}
        </>
      )}
    </>
  )
}

function IssueSearchList(_props: { searchIssueList?: Item[]; hideProject?: boolean }) {
  return (
    <>
      {/* <IssueList Issuelists={searchIssueList} /> */}
      {/* <IssueList Issuelists={issueList} /> <Pagination totalNum={100} pageSize={5} /> */}
    </>
  )
}

export const RightAvatar = ({ item }: { item: ItemsType[number] }) => {
  return (
    <>
      <div className='mr-10 flex w-fit items-center justify-between gap-10'>
        <div
          style={{
            visibility: `${item.comment_num === 0 ? 'hidden' : 'unset'}`
          }}
          className='flex items-center gap-2 text-sm text-gray-500'
        >
          <ChatBubbleIcon />
          <span>{item.comment_num}</span>
        </div>

        <div className='min-w-45'>
          <MemberHoverAvatarList users={item} />
        </div>
      </div>
    </>
  )
}
