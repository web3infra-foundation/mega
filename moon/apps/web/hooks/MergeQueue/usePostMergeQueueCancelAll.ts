import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { PostApiMergeQueueCancelAllData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostMergeQueueCancelAll() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.postApiMergeQueueCancelAll()

  return useMutation<PostApiMergeQueueCancelAllData, Error, RequestParams>({
    mutationFn: (params) => mutation.request(params),
    onSuccess: (response) => {
      if (response.data?.success) {
        toast.success(response.data.message || 'All pending items cancelled successfully')

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueList().requestKey()
        })

        queryClient.invalidateQueries({
          queryKey: legacyApiClient.v1.getApiMergeQueueStats().requestKey()
        })
      } else {
        toast.error('Failed to cancel all items')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to cancel all pending items')
    }
  })
}
