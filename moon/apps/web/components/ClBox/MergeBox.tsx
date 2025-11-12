import React, { useCallback, useMemo, useState } from 'react'
import { FeedMergedIcon } from '@primer/octicons-react'
import { useQueryClient } from '@tanstack/react-query'
import { useRouter } from 'next/router'

import { LoadingSpinner } from '@gitmono/ui'

import { useGetMergeBox } from '@/components/ClBox/hooks/useGetMergeBox'
import { useGetClReviewers } from '@/hooks/CL/useGetClReviewers'
import { usePostClMerge } from '@/hooks/CL/usePostClMerge'
import { usePostClReviewerApprove } from '@/hooks/CL/usePostClReviewerApprove'
import { useGetCurrentUser } from '@/hooks/useGetCurrentUser'
import { legacyApiClient } from '@/utils/queryClient'

import { ChecksSection } from './ChecksSection'
import { useMergeChecks } from './hooks/useMergeChecks'
import { MergeSection } from './MergeSection'
import { ReviewerSection } from './ReviewerSection'

export const MergeBox = React.memo<{ prId: string }>(({ prId }) => {
  const { checks, refresh } = useMergeChecks(prId)
  const [hasCheckFailures, setHasCheckFailures] = useState(true)
  const { mutate: approveCl, isPending: clMergeIsPending } = usePostClMerge(prId)
  const { mutate: reviewApprove } = usePostClReviewerApprove()
  const queryClient = useQueryClient()

  const route = useRouter()
  const { link } = route.query
  const id = typeof link === 'string' ? link : ''
  const { reviewers, isLoading: isReviewerLoading } = useGetClReviewers(id)

  const required: number = useMemo(() => reviewers.length, [reviewers])
  const actual: number = useMemo(() => reviewers.filter((i) => i.approved).length, [reviewers])
  const isAllReviewerApproved: boolean = useMemo(() => actual >= required, [actual, required])

  let isNowUserApprove: boolean | undefined = undefined
  const { data } = useGetCurrentUser()
  const find_user = reviewers.find((i) => i.username === data?.username)

  if (find_user) {
    isNowUserApprove = find_user.approved
  }

  const { mergeBoxData, isLoading: isAdditionLoading } = useGetMergeBox(prId)

  // 定义最终的合并处理函数
  const handleMerge = useCallback(async () => {
    // console.log('Final validation before merge...');
    // TODO: 再次发送校验请求
    refresh()

    // 模拟校验结果
    const stillHasFailures = false

    if (stillHasFailures) {
      alert('阻止合并：仍有检查项未通过，请刷新页面查看详情。')
    } else {
      // console.log('All checks passed. Sending change_list to backend...');

      approveCl(undefined)

      alert('合并请求已发送！')
    }
  }, [approveCl, refresh])

  const handleApprove = useCallback(async () => {
    reviewApprove(
      {
        link: id,
        data: {
          approved: true
        }
      },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({
            queryKey: legacyApiClient.v1.getApiClReviewers().requestKey(id)
          })
        }
      }
    )
  }, [reviewApprove, id, queryClient])

  const additionalChecks = mergeBoxData?.merge_requirements?.conditions ?? []

  return (
    <div className='flex'>
      <FeedMergedIcon size={24} className='ml-1 text-gray-500' />
      {isReviewerLoading && isAdditionLoading ? (
        <div className='flex h-[400px] items-center justify-center'>
          <LoadingSpinner />
        </div>
      ) : (
        <div className='ml-3 w-full divide-y rounded-lg border bg-white'>
          <ReviewerSection required={required} actual={actual} />
          <ChecksSection checks={checks} onStatusChange={setHasCheckFailures} additionalChecks={additionalChecks} />
          <MergeSection
            isNowUserApprove={isNowUserApprove}
            isAllReviewerApproved={isAllReviewerApproved}
            hasCheckFailures={hasCheckFailures}
            onMerge={handleMerge}
            onApprove={handleApprove}
            isMerging={clMergeIsPending}
          />
        </div>
      )}
    </div>
  )
})

MergeBox.displayName = 'MergeBox'
