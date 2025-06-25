'use client'

import React, { useCallback, useEffect, useState } from 'react'
import { formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'

import { ChatBubbleIcon, CheckCircleFilledFlushIcon, CircleFilledCloseIcon, ClockIcon } from '@gitmono/ui'
import { Link } from '@gitmono/ui/Link'
import { cn } from '@gitmono/ui/src/utils'

import { IssueIndexTabFilter as MRIndexTabFilter } from '@/components/Issues/IssueIndex'
import { ListBanner, ListItem as MrItem, IssueList as MrList } from '@/components/Issues/IssueList'
import { useScope } from '@/contexts/scope'
import { usePostMrList } from '@/hooks/usePostMrList'
import { apiErrorToast } from '@/utils/apiErrorToast'

import { IndexPageContainer, IndexPageContent } from '../IndexPages/components'
import { Pagination } from '../Issues/Pagenation'
import { filterAtom } from '../Issues/utils/store'
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
        return <CircleFilledCloseIcon color='#378f50' />
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

  return (
    <div className='m-4'>
      <Heading>Merge Request</Heading>
      <br />
      <IndexPageContainer>
        <IndexPageContent id='/[org]/mr' className={cn('@container', '3xl:max-w-7xl max-w-7xl')}>
          <MrList
            isLoading={isLoading}
            Issuelists={mrList}
            header={<ListBanner pickerTypes={[]} tabfilter={<MRIndexTabFilter part='mr' />} />}
          >
            {(issueList) => {
              return issueList.map((i) => (
                <Link
                  key={i.link}
                  href={{
                    pathname: `/${scope}/mr/${i.link}`,
                    query: {
                      title: i.title
                    }
                  }}
                >
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
