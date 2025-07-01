'use client'

import React, { useCallback, useEffect, useState } from 'react'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'

import { SyncOrganizationMember as Member } from '@gitmono/types/generated'
import {
  Button,
  ChatBubbleIcon,
  CheckCircleFilledFlushIcon,
  ChevronDownIcon,
  CircleFilledCloseIcon,
  ClockIcon
} from '@gitmono/ui'
import { Link } from '@gitmono/ui/Link'
import { cn } from '@gitmono/ui/src/utils'

import { IssueIndexTabFilter as MRIndexTabFilter } from '@/components/Issues/IssueIndex'
import {
  Dropdown,
  DropdownItemwithAvatar,
  DropdownItemwithLabel,
  ListBanner,
  ListItem as MrItem,
  IssueList as MrList
} from '@/components/Issues/IssueList'
import { useScope } from '@/contexts/scope'
import { usePostMrList } from '@/hooks/usePostMrList'
import { useSyncedMembers } from '@/hooks/useSyncedMembers'
import { apiErrorToast } from '@/utils/apiErrorToast'

import { IndexPageContainer, IndexPageContent } from '../IndexPages/components'
import { Label } from '../Issues/IssuesContent'
import { Pagination } from '../Issues/Pagenation'
import { tags } from '../Issues/utils/consts'
import { generateAllMenuItems, MenuConfig } from '../Issues/utils/generateAllMenuItems'
import { filterAtom, sortAtom } from '../Issues/utils/store'
import { Heading } from './catalyst/heading'

interface MrInfoItem {
  link: string
  title: string
  status: string
  open_timestamp: number
  merge_timestamp: number | null
  updated_at: number
}

export default function MrView() {
  const { scope } = useScope()
  const [mrList, setMrList] = useState<MrInfoItem[]>([])
  const [numTotal, setNumTotal] = useState(0)
  const [pageSize] = useState(10)
  const [status, _setStatus] = useAtom(filterAtom({ scope, part: 'mr' }))
  // const [status, _setStatus] = useState('open')
  const [page, _setPage] = useState(1)
  const [isLoading, setIsLoading] = useState(false)
  const { mutate: fetchMrList } = usePostMrList()

  const loadMrList = useCallback(() => {
    setIsLoading(true)
    fetchMrList(
      {
        data: {
          pagination: {
            page,
            per_page: pageSize
          },
          additional: {
            status
          }
        }
      },
      {
        onSuccess: (response) => {
          const data = response.data

          setMrList(
            data?.items?.map((item) => ({
              ...item,
              merge_timestamp: item.merge_timestamp ?? null
            })) ?? []
          )
          setNumTotal(data?.total ?? 0)
        },
        onError: apiErrorToast,
        onSettled: () => setIsLoading(false)
      }
    )
  }, [page, pageSize, status, fetchMrList])

  useEffect(() => {
    loadMrList()
  }, [loadMrList])

  // const getStatusTag = (status: string) => {
  //   const normalizedStatus = status.toLowerCase()

  //   switch (normalizedStatus) {
  //     case 'open':
  //       return <Tag color='success'>open</Tag>
  //     case 'merged':
  //       return <Tag color='purple'>merged</Tag>
  //     case 'closed':
  //       return <Tag color='error'>closed</Tag>
  //     default:
  //       return null
  //   }
  // }

  const getStatusIcon = (status: string) => {
    const normalizedStatus = status.toLowerCase()

    switch (normalizedStatus) {
      case 'open':
        return <CircleFilledCloseIcon color='#f44613' />
      case 'closed':
        return <ClockIcon size={16} />
      case 'merged':
        return <CheckCircleFilledFlushIcon color='#378f50' size={16} />
      default:
        return null
    }
  }

  const getDescription = (item: MrInfoItem) => {
    const normalizedStatus = item.status.toLowerCase()

    switch (normalizedStatus) {
      case 'open':
        return `MergeRequest opened by Admin ${formatDistance(fromUnixTime(item.open_timestamp), new Date(), { addSuffix: true })} `
      case 'merged':
        if (item.merge_timestamp !== null) {
          return `MergeRequest merged by Admin ${formatDistance(fromUnixTime(item.merge_timestamp), new Date(), { addSuffix: true })}`
        } else {
          return ''
        }
      case 'closed':
        return `MR ${item.link} closed by Admin ${formatDistance(fromUnixTime(item.updated_at), new Date(), { addSuffix: true })}`
      default:
        return null
    }
  }

  const [sort, setSort] = useAtom(sortAtom({ scope, filter: 'sortPickerMR' }))
  const { members } = useSyncedMembers()

  const MemberConfig: MenuConfig<Member>[] = [
    {
      key: 'Author',
      isChosen: (item) => item.user.id === sort['Author'],
      onSelectFactory: (item: Member) => (e: Event) => {
        e.preventDefault()
        if (item.user.id === sort['Author']) {
          loadMrList()
          setSort({
            ...sort,
            Author: ''
          })
        } else {
          setMrList(mrList.filter((i) => i.link === sort['Author']))
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
          loadMrList()

          setSort({
            ...sort,
            Assignees: ''
          })
        } else {
          setMrList(mrList.filter((i) => i.link === sort['Assignees']))
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

  const handleOpen = (open: boolean) => {
    if (open) {
      // open: do nothing
    } else {
      // close: fetch data from labels array
    }
  }

  const member = generateAllMenuItems(members, MemberConfig)

  const labels = generateAllMenuItems(tags, LabelConfig)

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

  return (
    <div className='m-4'>
      <Heading>Merge Request</Heading>
      <br />
      <IndexPageContainer>
        <IndexPageContent id='/[org]/mr' className={cn('@container', '3xl:max-w-7xl max-w-7xl')}>
          <MrList
            isLoading={isLoading}
            Issuelists={mrList}
            header={
              <ListBanner
                pickerTypes={['Author', 'Labels', 'Projects', 'Milestones', 'Assignees', 'Types']}
                tabfilter={<MRIndexTabFilter part='mr' />}
              >
                {(p) => ListHeaderItem(p)}
              </ListBanner>
            }
          >
            {(issueList) => {
              return issueList.map((i) => (
                <Link key={i.link} href={`/${scope}/mr/${i.link}`}>
                  <MrItem
                    title={i.title}
                    leftIcon={getStatusIcon(i.status)}
                    rightIcon={<ChatBubbleIcon />}
                  >
                    <div className='text-xs text-[#59636e]'>
                      {i.link} {i.status} {getDescription(i)}
                    </div>
                  </MrItem>
                </Link>
              ))
            }}
          </MrList>
          <Pagination totalNum={numTotal} pageSize={pageSize} />
        </IndexPageContent>
      </IndexPageContainer>
    </div>
  )
}
