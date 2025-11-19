import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { PostApiMergeQueueRetryByClLinkData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostMergeQueueRetry() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.postApiMergeQueueRetryByClLink()

  return useMutation<PostApiMergeQueueRetryByClLinkData, Error, string & RequestParams>({
    mutationFn: (clLink) => mutation.request(clLink),
    onSuccess: (response, clLink) => {
      if (response.data?.success) {
        toast.success(response.data.message || 'Retry initiated successfully')

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
        toast.error('Failed to retry')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to retry merge')
    }
  })
}
