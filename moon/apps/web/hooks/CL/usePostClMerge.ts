import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { PostApiClMergeData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClMerge(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiClMergeData, Error, void>({
    mutationKey: legacyApiClient.v1.postApiClMerge().requestKey(link),
    mutationFn: () => legacyApiClient.v1.postApiClMerge().request(link, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(link)
      })
    }
  })
}
