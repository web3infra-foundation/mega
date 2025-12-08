import React, { useEffect, useMemo, useState } from 'react'
import { CopyIcon, FileCodeIcon } from '@primer/octicons-react'
import copy from 'copy-to-clipboard'
import { format, formatDistance, fromUnixTime } from 'date-fns'
import { useAtom } from 'jotai'
import { useRouter } from 'next/router'
import toast from 'react-hot-toast'

import { CommitSummary, CommonPageDiffItem, DiffItem } from '@gitmono/types'
import { LoadingSpinner } from '@gitmono/ui'

import { formatAssignees } from '@/components/CodeView/CommitsView'
import { commitPath } from '@/components/CodeView/CommitsView/items'
import FileDiff from '@/components/DiffView/FileDiff'
import { MemberHoverAvatarList } from '@/components/Issues/MemberHoverAvatarList'
import { useScope } from '@/contexts/scope'
import { useGetCommitsDetail } from '@/hooks/commits/useGetCommitsDetail'

interface CommitDetailData {
  commit: CommitSummary
  diffs: DiffItem[]
}

export const CommitsDetailView: React.FC = () => {
  const router = useRouter()
  const { sha } = router.query
  const { scope } = useScope()

  const { mutate: commitDetail, isPending: isLoadingCommitDetail } = useGetCommitsDetail()

  const [commitPathValue, _setCommitPath] = useAtom(commitPath)
  const [commitsDetail, setCommitsDetail] = useState<CommitDetailData>()

  useEffect(() => {
    if (!sha || !commitPathValue) return

    commitDetail(
      {
        data: {
          path: commitPathValue,
          sha: Array.isArray(sha) ? sha[0] : (sha ?? '')
        }
      },
      {
        onSuccess: (response) => {
          const data = response.data

          if (data) {
            setCommitsDetail(data)
          }
        }
      }
    )
  }, [sha, commitDetail, commitPathValue])

  // Convert diffs to CommonPageDiffItem format
  const fileChangeData: CommonPageDiffItem | undefined = useMemo(() => {
    if (!commitsDetail?.diffs) return undefined

    return {
      items: commitsDetail.diffs,
      total: commitsDetail.diffs.length
    }
  }, [commitsDetail])

  // Calculate total stats
  const stats = useMemo(() => {
    if (!commitsDetail?.diffs) return { additions: 0, deletions: 0, files: 0 }

    let additions = 0
    let deletions = 0

    commitsDetail.diffs.forEach((diff) => {
      const lines = diff.data.split('\n')

      lines.forEach((line) => {
        if (line.startsWith('+') && !line.startsWith('+++')) {
          additions++
        } else if (line.startsWith('-') && !line.startsWith('---')) {
          deletions++
        }
      })
    })

    return {
      additions,
      deletions,
      files: commitsDetail.diffs.length
    }
  }, [commitsDetail])

  const commitSha = Array.isArray(sha) ? sha[0] : (sha ?? '')
  const commitDate = commitsDetail?.commit.date ? fromUnixTime(parseInt(commitsDetail.commit.date, 10)) : null

  if (isLoadingCommitDetail && !commitsDetail) {
    return (
      <div className='flex h-[400px] items-center justify-center'>
        <LoadingSpinner />
      </div>
    )
  }

  if (!commitsDetail) {
    return (
      <div className='flex h-[400px] items-center justify-center'>
        <div className='text-center'>
          <p className='text-lg text-gray-600'>No commit data found</p>
        </div>
      </div>
    )
  }

  return (
    <div className='flex flex-col'>
      {/* Commit Header */}
      <div className='border-gray-200 bg-white px-5 py-4 text-2xl'>
        <div className='flex items-center justify-between'>
          <h1 className='mb-3 pt-5 font-semibold text-gray-900'>
            <span className='pr-2'>Commit</span>
            <code className='rounded bg-gray-100 px-2 py-1 text-gray-800'>
              {commitsDetail.commit.sha.substring(0, 7)}
            </code>
          </h1>

          <div
            className='flex items-center gap-1.5 rounded-md border bg-gray-100 px-2 py-1 hover:bg-gray-200'
            onClick={() => {
              router.push(`/${scope}/code`)
            }}
          >
            <FileCodeIcon size={16} />
            <span className='text-sm'>Browse files</span>
          </div>
        </div>

        <div className='flex flex-wrap items-center gap-2 text-sm text-gray-600'>
          <div className='flex items-center gap-2'>
            <MemberHoverAvatarList authors={[commitsDetail.commit.author]} isLeft={true} />
            {formatAssignees([commitsDetail.commit.author])}
            <span>authored</span>
            {commitDate && (
              <time dateTime={commitDate.toISOString()} title={format(commitDate, 'PPpp')}>
                {formatDistance(commitDate, new Date(), { addSuffix: true })}
              </time>
            )}
            {'  ·'}
          </div>

          {commitDate && (
            <span className='inline-flex items-center rounded-full border border-gray-300 bg-gray-50 px-2 py-0.5 text-[11px] font-medium text-gray-700'>
              GPG Verified{/*{item.Verified}*/}
            </span>
          )}
        </div>
      </div>

      {/* File Stats */}
      {fileChangeData && (
        <div className='flex flex-col px-5'>
          <div className='justify-center rounded-lg border border-gray-200'>
            <div className='border-b px-4 py-2 text-gray-900'> {commitsDetail.commit.short_message}</div>

            <div className='flex items-center justify-between px-4 py-2 text-sm'>
              {/* Left side: File stats */}
              <div className='flex items-center gap-4'>
                <div className='flex items-center gap-2'>
                  <span className='font-medium text-gray-700'>{stats.files}</span>
                  <span className='text-gray-600'>file{stats.files !== 1 ? 's' : ''} changed</span>
                </div>
                <div className='flex items-center gap-2'>
                  <span className='font-medium text-green-600'>+{stats.additions}</span>
                  <span className='text-gray-600'>additions</span>
                </div>
                <div className='flex items-center gap-2'>
                  <span className='font-medium text-red-600'>-{stats.deletions}</span>
                  <span className='text-gray-600'>deletions</span>
                </div>
              </div>

              {/* Right side: Parent commits and current commit */}
              <div className='flex items-center gap-2 text-sm text-gray-600'>
                {commitsDetail.commit.parents && commitsDetail.commit.parents.length > 0 ? (
                  <>
                    <span>
                      {commitsDetail.commit.parents.length} parent{commitsDetail.commit.parents.length > 1 ? 's' : ''}
                    </span>
                    {commitsDetail.commit.parents.map((parentSha, index) => (
                      <React.Fragment key={parentSha}>
                        <button
                          onClick={() => router.push(`/${scope}/code/commit/${parentSha}`)}
                          className='font-mono text-gray-800 underline hover:underline'
                        >
                          {parentSha.substring(0, 7)}
                        </button>
                        {index < commitsDetail.commit.parents.length - 1 && <span>,</span>}
                      </React.Fragment>
                    ))}
                    <span>commit</span>
                    <code className='font-mono text-gray-800'>{commitSha.substring(0, 7)}</code>
                    <button
                      onClick={() => {
                        if (copy(commitSha)) {
                          toast.success('Copied to clipboard')
                        } else {
                          toast.error('Copy failed')
                        }
                      }}
                      className='text-gray-400 hover:text-gray-600'
                      title='Copy full SHA'
                    >
                      <CopyIcon className='h-4 w-4' />
                    </button>
                  </>
                ) : (
                  <>
                    <span>commit</span>
                    <code className='font-mono text-gray-800'>{commitSha.substring(0, 7)}</code>
                    <button
                      onClick={() => {
                        if (copy(commitSha)) {
                          toast.success('Copied to clipboard')
                        } else {
                          toast.error('Copy failed')
                        }
                      }}
                      className='text-gray-400 hover:text-gray-600'
                      title='Copy full SHA'
                    >
                      <CopyIcon className='h-4 w-4' />
                    </button>
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* File Diffs */}
      {fileChangeData ? (
        <FileDiff
          fileChangeData={fileChangeData}
          fileChangeIsLoading={isLoadingCommitDetail}
          treeData={{
            data: [],
            err_message: '',
            req_result: true
          }}
          treeIsLoading={false}
        />
      ) : (
        <div className='flex h-[200px] items-center justify-center text-gray-500'>No file changes</div>
      )}
    </div>
  )
}

export default CommitsDetailView
