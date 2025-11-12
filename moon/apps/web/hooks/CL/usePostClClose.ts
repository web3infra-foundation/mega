import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { PostApiClCloseData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClClose(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiClCloseData, Error>({
    mutationKey: legacyApiClient.v1.postApiClClose().requestKey(link),
    mutationFn: () => legacyApiClient.v1.postApiClClose().request(link, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(link)
      })
    }
  })
}
