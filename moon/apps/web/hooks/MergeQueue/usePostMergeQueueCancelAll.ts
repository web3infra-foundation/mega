import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { PostApiMergeQueueCancelAllData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Hook to cancel all pending items in the merge queue.
 *
 * @returns A mutation object that cancels all pending merge queue items.
 * The mutation accepts optional request parameters and invalidates queue-related queries on success.
 *
 * @example
 * ```tsx
 * const { mutate: cancelAll } = usePostMergeQueueCancelAll()
 * cancelAll() // Cancel all pending items
 * ```
 */
export function usePostMergeQueueCancelAll() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.postApiMergeQueueCancelAll()

  return useMutation<PostApiMergeQueueCancelAllData, Error, RequestParams>({
    mutationFn: (params) => mutation.request(params),
    onSuccess: (response) => {
      if (response.req_result) {
        toast.success('All pending items cancelled successfully')

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueList().requestKey()
        })

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueStats().requestKey()
        })
      } else {
        toast.error(response.err_message || 'Failed to cancel all items')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to cancel all pending items')
    }
  })
}
