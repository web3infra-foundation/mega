import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { PostApiMrCloseData, RequestParams } from '@gitmono/types'

export function usePostMrClose(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiMrCloseData, Error>({
    mutationKey: legacyApiClient.v1.postApiMrClose().requestKey(link),
    mutationFn: () => legacyApiClient.v1.postApiMrClose().request(link, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(link),
      })
    },
  })
}