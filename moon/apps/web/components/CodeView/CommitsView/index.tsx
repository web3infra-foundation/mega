import React, { useEffect, useMemo, useState } from 'react'
import { ChevronUpDownIcon } from '@heroicons/react/24/solid'
import { CopyIcon } from '@primer/octicons-react'
import copy from 'copy-to-clipboard'
import { format, formatDistance, fromUnixTime } from 'date-fns'
import { useSetAtom } from 'jotai'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { GitCommitIcon } from '@gitmono/ui'

import { ListBanner } from '@/components/ClView/ClList'
import { MemberHovercard } from '@/components/InlinePost/MemberHovercard'
import { MemberHoverAvatarList } from '@/components/Issues/MemberHoverAvatarList'
import { Pagination } from '@/components/Issues/Pagenation'
import { useScope } from '@/contexts/scope'
import { useGetCommitsHistory } from '@/hooks/commits/useGetCommitsHistory'

import { AuthorDropdown, BranchDropdown, DateRangeValue, TimeDropdown } from './dropdown'
import { commitPath, CommitsItem, CommitsList } from './items'
import { mockMembers } from './mock'

const formatCommitDate = (timestamp: number): string => {
  const dateObject = fromUnixTime(timestamp)

  return format(dateObject, 'MMM d, yyyy')
}

export const formatAssignees = (assignees: string[]): React.ReactNode => {
  if (assignees.length === 1) {
    return (
      <MemberHovercard username={assignees[0]}>
        <span className='cursor-pointer hover:text-blue-600 hover:underline'>{assignees[0]}</span>
      </MemberHovercard>
    )
  } else if (assignees.length === 2) {
    return (
      <>
        <MemberHovercard username={assignees[0]}>
          <span className='cursor-pointer hover:text-blue-600 hover:underline'>{assignees[0]}</span>
        </MemberHovercard>
        {' and '}
        <MemberHovercard username={assignees[1]}>
          <span className='cursor-pointer hover:text-blue-600 hover:underline'>{assignees[1]}</span>
        </MemberHovercard>
      </>
    )
  } else if (assignees.length >= 3) {
    return `${assignees.length} people`
  }
  return ''
}

type Commits = {
  author: string
  committer: string
  date: string
  parents: string[]
  sha: string
  short_message: string
}[]

export const CommitsView: React.FC = () => {
  const { scope } = useScope()

  const members = mockMembers
  const setCommitPath = useSetAtom(commitPath)
  const { mutate: commitslist, isPending: isLoadingCommits } = useGetCommitsHistory()
  const [commitsList, setCommitsList] = useState<Commits>([])
  const [numTotal, setNumTotal] = useState(0)

  // Filter states
  const [branchState, setBranchState] = useState<string>('main')
  const [authorState, setAuthorState] = useState<string>('')
  const [timeState, setTimeState] = useState<DateRangeValue>({ from: undefined, to: undefined })

  const [pageNum, setPageNum] = useState<number>(1)
  const [pageSize] = useState(35)

  const router = useRouter()
  const { path } = router.query
  const fullPath = Array.isArray(path) ? path.join('/') : path || ''

  // Handle filter close callback
  const handleFilterClose = (_value: unknown) => {
    // Can be used for analytics or additional logic when filter closes
  }

  const groupedCommits = useMemo((): [string, Commits][] => {
    const grouped = new Map<string, Commits>()

    for (const commit of commitsList) {
      const dateObject = fromUnixTime(parseInt(commit.date, 10))
      const dateKey = format(dateObject, 'MMM d, yyyy')

      if (!grouped.has(dateKey)) {
        grouped.set(dateKey, [])
      }
      grouped.get(dateKey)?.push(commit)
    }

    return Array.from(grouped.entries())
  }, [commitsList])

  useEffect(() => {
    commitslist(
      {
        data: {
          additional: {
            path: fullPath,
            author: authorState,
            refs: branchState
          },
          pagination: {
            page: pageNum,
            per_page: pageSize
          }
        }
      },
      {
        onSuccess: (response) => {
          const data = response.data

          setCommitPath(fullPath)
          setCommitsList((data?.items ?? []) as Commits)
          setNumTotal(data?.total ?? 0)
        }
      }
    )
  }, [authorState, branchState, commitslist, fullPath, pageNum, pageSize, setCommitPath])

  return (
    <>
      <CommitsList
        isLoading={isLoadingCommits}
        lists={groupedCommits}
        header={
          <ListBanner
            tabfilter={<BranchDropdown value={branchState} onChange={setBranchState} onClose={handleFilterClose} />}
          >
            <div className='flex gap-2'>
              <AuthorDropdown
                members={members}
                value={authorState}
                onChange={setAuthorState}
                onClose={handleFilterClose}
              />
              <TimeDropdown members={members} value={timeState} onChange={setTimeState} onClose={handleFilterClose} />
            </div>
          </ListBanner>
        }
      >
        {(groupedCommits: [string, Commits][]) => {
          return groupedCommits.map(([dateKey, dailyCommits]) => (
            <div key={dateKey}>
              <div className='relative'>
                {dailyCommits.map((item, index) => {
                  const commitDate = formatCommitDate(Number(item.date))

                  return (
                    <div key={item.sha} className='relative flex gap-4'>
                      <div className='relative flex w-5 flex-col items-center'>
                        <div className='absolute bottom-0 left-1/2 top-0 w-0.5 -translate-x-1/2 transform bg-gray-200' />
                        {index === 0 && (
                          <div className='relative z-10 mt-3 flex-shrink-0 rounded-full bg-white'>
                            <GitCommitIcon size={18} className='text-gray-400' />
                          </div>
                        )}
                      </div>

                      <div className='flex flex-1 flex-col'>
                        <div className='py-3 text-sm text-gray-600'>Commits on {commitDate}</div>

                        <div className='flex-1'>
                          <CommitsItem
                            key={item.sha}
                            title={item.short_message}
                            labels={
                              <span className='inline-flex items-center rounded-full border border-gray-300 bg-gray-50 px-2 py-0.5 text-[11px] font-medium text-gray-700'>
                                GPG Verified{/*{item.Verified}*/}
                              </span>
                            }
                            gitId={
                              <span className='items-center font-mono text-xs text-gray-500'>
                                {item.sha.substring(0, 7)}
                              </span>
                            }
                            copyIcon={
                              <button
                                onClick={(e) => {
                                  e.stopPropagation()
                                  copy(item.sha) ? toast.success('Copied to clipboard') : toast.error('Copy failed')
                                }}
                                className='text-gray-400 hover:text-gray-600'
                              >
                                <CopyIcon className='h-4 w-4' />
                              </button>
                            }
                            rightIcon={<ChevronUpDownIcon className='h-5 w-5 rotate-90 text-gray-500' />}
                            onClick={() => {
                              router.push(`/${scope}/code/commit/${item.sha}`)
                            }}
                          >
                            <div className='flex items-center gap-2 text-xs leading-4 text-gray-500'>
                              <div className='h-5 flex-shrink-0'>
                                <MemberHoverAvatarList authors={[item.committer]} isLeft={true} />
                              </div>
                              <div className='flex items-center gap-1 whitespace-nowrap'>
                                <span>{formatAssignees([item.committer])}</span>
                                <span className='text-[11px] text-gray-400'>
                                  authored{' '}
                                  {formatDistance(fromUnixTime(parseInt(item.date, 10)), new Date(), {
                                    addSuffix: true
                                  })}
                                </span>
                              </div>
                            </div>
                          </CommitsItem>
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          ))
        }}
      </CommitsList>
      <Pagination
        totalNum={numTotal}
        currentPage={pageNum}
        pageSize={pageSize}
        onChange={(page: number) => setPageNum(page)}
      />
    </>
  )
}

export default CommitsView
