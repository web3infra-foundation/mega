import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { DeleteApiMergeQueueRemoveByClLinkData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useDeleteMergeQueueRemove() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.deleteApiMergeQueueRemoveByClLink()

  return useMutation<DeleteApiMergeQueueRemoveByClLinkData, Error, string & RequestParams>({
    mutationFn: (clLink) => mutation.request(clLink),
    onSuccess: (response, clLink) => {
      if (response.data?.success) {
        toast.success(response.data.message || 'Removed from queue successfully')

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
        toast.error('Failed to remove from queue')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to remove from queue')
    }
  })
}
