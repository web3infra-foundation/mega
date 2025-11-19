import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { PostApiMergeQueueAddData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Hook to add a change list (CL) to the merge queue.
 *
 * @returns A mutation object that adds a CL to the merge queue.
 * The mutation accepts an object containing the CL link and optional request parameters,
 * and invalidates related queries on success.
 *
 * @example
 * ```tsx
 * const { mutate: addToQueue } = usePostMergeQueueAdd()
 * addToQueue({ cl_link: 'cl/123' })
 * ```
 */
export function usePostMergeQueueAdd() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.postApiMergeQueueAdd()

  return useMutation<PostApiMergeQueueAddData, Error, { cl_link: string } & RequestParams>({
    mutationFn: (data) => mutation.request(data),
    onSuccess: (response, variables) => {
      if (response.req_result && response.data?.success) {
        toast.success(response.data.message || 'Added to merge queue successfully')

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueList().requestKey()
        })

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueStats().requestKey()
        })

        if (variables.cl_link) {
          queryClient.invalidateQueries({
            queryKey: legacyApiClient.v1.getApiMergeQueueStatusByClLink().requestKey(variables.cl_link)
          })
        }
      } else {
        toast.error(response.err_message || response.data?.message || 'Failed to add to queue')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to add to merge queue')
    }
  })
}
