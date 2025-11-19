import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'react-hot-toast'

import type { PostApiMergeQueueAddData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostMergeQueueAdd() {
  const queryClient = useQueryClient()
  const mutation = legacyApiClient.v1.postApiMergeQueueAdd()

  return useMutation<PostApiMergeQueueAddData, Error, { cl_link: string } & RequestParams>({
    mutationFn: (data) => mutation.request(data),
    onSuccess: (response, variables) => {
      if (response.data?.success) {
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
        toast.error('Failed to add to queue')
      }
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to add to merge queue')
    }
  })
}
