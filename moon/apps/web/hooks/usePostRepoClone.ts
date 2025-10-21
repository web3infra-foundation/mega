import { useMutation, useQueryClient } from '@tanstack/react-query'
import toast from 'react-hot-toast'

import { CloneRepoPayload, PostApiRepoCloneData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostRepoClone(params?: RequestParams) {
  const qc = useQueryClient()

  return useMutation<PostApiRepoCloneData, Error, CloneRepoPayload>({
    mutationFn: (data) => legacyApiClient.v1.postApiRepoClone().request(data, params),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: legacyApiClient.v1.getApiTree().baseKey })
      toast.success('Repository sync started successfully!')
    },
    onError: (error) => {
      toast.error(error?.message || 'Failed to sync repository')
    }
  })
}
