import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { PostApiMrMergeData, RequestParams } from '@gitmono/types'

export function usePostMrMerge(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiMrMergeData, Error, void>({
    mutationKey: legacyApiClient.v1.postApiMrMerge().requestKey(link),
    mutationFn: () =>
      legacyApiClient.v1.postApiMrMerge().request(link, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(link),
      })
    },
  })
}

