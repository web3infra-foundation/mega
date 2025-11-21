import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { DeleteApiMergeQueueRemoveByClLinkData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Hook to remove a change list (CL) from the merge queue.
 *
 * @returns A mutation object that removes a CL from the queue by its link.
 * The mutation accepts a CL link string and invalidates related queries on success.
 *
 * @example
 * ```tsx
 * const { mutate: removeFromQueue } = useDeleteMergeQueueRemove()
 * removeFromQueue('cl/123')
 * ```
 */
export function useDeleteMergeQueueRemove() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.deleteApiMergeQueueRemoveByClLink()

  return useMutation<DeleteApiMergeQueueRemoveByClLinkData, Error, string>({
    mutationFn: (clLink) => mutation.request(clLink),
    onSuccess: (response, clLink) => {
      if (response.req_result) {
        toast.success('Removed from queue successfully')

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
        toast.error(response.err_message || 'Failed to remove from queue')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to remove from queue')
    }
  })
}
