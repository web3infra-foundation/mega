import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { PostApiMrReopenData, RequestParams } from '@gitmono/types'

export function usePostMrReopen(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiMrReopenData, Error>({
    mutationKey: legacyApiClient.v1.postApiMrReopen().requestKey(link),
    mutationFn: () => legacyApiClient.v1.postApiMrReopen().request(link, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(link),
      })
    },
  })
}