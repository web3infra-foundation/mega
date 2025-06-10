import React, { useCallback, useEffect, useState } from 'react'
import { useInfiniteQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'

import { PostApiIssueListData } from '@gitmono/types/generated'

import { EmptySearchResults } from '@/components/Feed/EmptySearchResults'
import { IssueList } from '@/components/Issues/IssueList'
import { filterAtom } from '@/components/Issues/utils/store'
import { useScope } from '@/contexts/scope'
import { useGetIssueLists } from '@/hooks/issues/useGetIssueLists'
import { apiErrorToast } from '@/utils/apiErrorToast'

import { IndexPageInstantLoading } from '../IndexPages/components'
import { IssueIndexEmptyState } from './IssueIndex'
import { Pagination } from './Pagenation'

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

export function IssuesContent({ searching }: Props) {
  // TODO:rebuild bu useInfiniteQuery
  const { mutate: issueLists } = useGetIssueLists()
  const { scope } = useScope()

  const [pageSize, _setPageSize] = useState(10)

  const [status, _setStatus] = useAtom(filterAtom(scope))

  const [issueList, setIssueList] = useState<Item[]>([])

  const [loading, setLoading] = useState(false)

  const [numTotal, setNumTotal] = useState(0)

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

  if (!issueList.length) {
    return searching ? <EmptySearchResults /> : <IssueIndexEmptyState />
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
          <IssueList Issuelists={issueList} />
          <Pagination totalNum={numTotal} pageSize={pageSize} />
          {/* <Pagination totalNum={7} pageSize={5} /> */}
        </>
      )}
    </>
  )
}

function IssueSearchList({ searchIssueList }: { searchIssueList: Item[]; hideProject?: boolean }) {
  return (
    <>
      <IssueList Issuelists={searchIssueList} />
      {/* <IssueList Issuelists={issueList} /> <Pagination totalNum={100} pageSize={5} /> */}
    </>
  )
}
