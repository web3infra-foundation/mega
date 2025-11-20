import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { PostApiMergeQueueRetryByClLinkData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Hook to retry a failed merge queue item.
 *
 * @returns A mutation object that retries a merge operation for a specific CL.
 * The mutation accepts a CL link string and invalidates related queries on success.
 *
 * @example
 * ```tsx
 * const { mutate: retryMerge } = usePostMergeQueueRetry()
 * retryMerge('cl/123')
 * ```
 */
export function usePostMergeQueueRetry() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.postApiMergeQueueRetryByClLink()

  return useMutation<PostApiMergeQueueRetryByClLinkData, Error, string>({
    mutationFn: (clLink) => mutation.request(clLink),
    onSuccess: (response, clLink) => {
      if (response.req_result) {
        toast.success('Retry initiated successfully')

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueList().requestKey()
        })

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueStats().requestKey()
        })

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueStatusByClLink().requestKey(clLink)
        })
      } else {
        toast.error(response.err_message || 'Failed to retry')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to retry merge')
    }
  })
}
