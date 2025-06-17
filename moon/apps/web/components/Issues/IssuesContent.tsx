import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'

import { SyncOrganizationMember as Member, PostApiIssueListData } from '@gitmono/types/generated'
import { Button, ChatBubbleIcon, CheckCircleFilledFlushIcon, ChevronDownIcon, OrderedListIcon } from '@gitmono/ui'
import { Link } from '@gitmono/ui/Link'

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
import { filterAtom, sortAtom } from '@/components/Issues/utils/store'
import { useScope } from '@/contexts/scope'
import { useGetIssueLists } from '@/hooks/issues/useGetIssueLists'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'
import { apiErrorToast } from '@/utils/apiErrorToast'
import { atomWithWebStorage } from '@/utils/atomWithWebStorage'

import { IndexPageInstantLoading } from '../IndexPages/components'
import { Pagination } from './Pagenation'
import { orderTags, tags } from './utils/consts'
import { generateAllMenuItems, MenuConfig } from './utils/generateAllMenuItems'

interface Props {
  getIssues?: ReturnType<typeof useInfiniteQuery<PostApiIssueListData>>
  searching?: boolean
  hideProject?: boolean
}

export interface Item {
  closed_at?: number | null
  link: string
  owner: number
  title: string
  status: string
  open_timestamp: number
  updated_at: number
}

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

  const [status, _setStatus] = useAtom(filterAtom(scope))

  const [issueList, setIssueList] = useState<Item[]>([])

  const [loading, setLoading] = useState(false)

  const [numTotal, setNumTotal] = useState(0)

  const [sort, setSort] = useAtom(sortAtom({ scope, filter: 'sortPicker' }))

  const orderAtom = useMemo(() => atomWithWebStorage<string>(`${scope}:issue-order`, 'Newest'), [scope])

  const [order, _setOrder] = useAtom(orderAtom)

  const { members } = useSyncedMembers()

  const MemberConfig: MenuConfig<Member>[] = [
    {
      key: 'Author',
      isChosen: (item) => item.user.id === sort['Author'],
      onSelectFactory: (item: Member) => (e: Event) => {
        e.preventDefault()
        if (item.user.id === sort['Author']) {
          fetchData(1, pageSize)
          setSort({
            ...sort,
            Author: ''
          })
        } else {
          setIssueList(issueList.filter((i) => i.link === sort['Author']))
          setSort({
            ...sort,
            Author: item.user.id
          })
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item: Member) => <DropdownItemwithAvatar member={item} classname='text-sm' />
    },
    {
      key: 'Assignees',
      isChosen: (item: Member) => item.user.id === sort['Assignees'],
      onSelectFactory: (item: Member) => (e: Event) => {
        e.preventDefault()
        if (item.user.id === sort['Assignees']) {
          fetchData(1, pageSize)

          setSort({
            ...sort,
            Assignees: ''
          })
        } else {
          setIssueList(issueList.filter((i) => i.link === sort['Assignees']))
          setSort({
            ...sort,
            Assignees: item.user.id
          })
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item: Member) => <DropdownItemwithAvatar member={item} classname='text-sm' />
    }
  ]

  const LabelConfig: MenuConfig<Label>[] = [
    {
      key: 'Labels',
      isChosen: (item) => sort['Labels']?.includes(item.id),

      onSelectFactory: (item: Label) => (e: Event) => {
        e.preventDefault()
        if (sort['Labels']?.includes(item.id)) {
          // fetchData(1, pageSize)
          // sort['Labels'] contains the id of each labels which are chosed
          setSort({
            ...sort,
            Labels: (sort['Labels'] as string[]).filter((i) => i !== item.id)
          })
        } else {
          // setIssueList(issueList.filter((i) => i.link === sort['Labels']))
          setSort({
            ...sort,
            // make sure labels must be an array of string
            Labels: [...((sort['Labels'] as string[]) ?? []), item.id]
          })
        }
      },
      className: 'overflow-hidden',
      labelFactory: (item: Label) => <DropdownItemwithLabel label={item} />
    }
  ]

  const OrderConfig: MenuConfig<string>[] = [
    {
      key: 'Order',
      isChosen: (item) => item === 'Oldest' || item === 'Newest',

      onSelectFactory: (_item: string) => (e: Event) => {
        e.preventDefault()
       
      },
      className: 'overflow-hidden',
      labelFactory: (item: string) => <div>{item}</div>
    }
  ]

  const member = generateAllMenuItems(members, MemberConfig)

  const labels = generateAllMenuItems(tags, LabelConfig)

  const orders = generateAllMenuItems(orderTags, OrderConfig)

  const handleOpen = (open: boolean) => {
    if (open) {
      // open: do nothing
    } else {
      // close: fetch data from labels array
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
            onOpen={(open) => handleOpen(open)}
            isChosen={!sort['Labels']?.length}
            key={p}
            name={p}
            dropdownArr={labels?.get('Labels').all}
            dropdownItem={labels?.get('Labels').chosen}
          />
        )
      case `${order}`:
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

  const fetchData = useCallback(
    (page: number, per_page: number) => {
      setLoading(true)
      issueLists(
        {
          data: { pagination: { page, per_page }, additional: { status } }
        },
        {
          onSuccess: (response) => {
            const data = response.data

            setIssueList(data?.items ?? [])
            setNumTotal(data?.total ?? 0)
          },
          onError: apiErrorToast,
          onSettled: () => setLoading(false)
        }
      )
    },

    [status, issueLists]
  )

  useEffect(() => {
    fetchData(1, pageSize)
  }, [pageSize, fetchData])

  if (loading) {
    return <IndexPageInstantLoading />
  }

  // if (!issueList.length) {
  //   return searching ? <EmptySearchResults /> : <IssueIndexEmptyState />
  // }
  if (!issueList.length && searching) {
    return <EmptySearchResults />
  }

  return (
    <>
      {/* TODO:Searching logic need to be completed */}
      {searching ? (
        <>
          <IssueSearchList searchIssueList={issueList} />
          <Pagination totalNum={numTotal} pageSize={pageSize} />
        </>
      ) : (
        <>
          <IssueList
            Issuelists={issueList}
            header={
              <ListBanner
                pickerTypes={['Author', 'Labels', 'Projects', 'Milestones', 'Assignees', 'Types', `${order}`]}
              >
                {(p) => ListHeaderItem(p)}
              </ListBanner>
            }
          >
            {(issueList) => {
              return issueList.map((i) => (
                <Link key={i.link} href={`/${scope}/issue/${i.link}`}>
                  <ListItem
                    key={i.link}
                    title={i.title}
                    leftIcon={<CheckCircleFilledFlushIcon color='#378f50' size={16} />}
                    rightIcon={<ChatBubbleIcon />}
                  >
                    <div className='text-xs text-[#59636e]'>
                      {i.link} · {i.owner} {i.status}{' '}
                      {formatDistance(fromUnixTime(i.open_timestamp), new Date(), { addSuffix: true })}
                    </div>
                  </ListItem>
                </Link>
              ))
            }}
          </IssueList>
          <Pagination totalNum={numTotal} pageSize={pageSize} />
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
