import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { PostApiClReopenData, RequestParams } from '@gitmono/types'

export function usePostClReopen(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiClReopenData, Error>({
    mutationKey: legacyApiClient.v1.postApiClReopen().requestKey(link),
    mutationFn: () => legacyApiClient.v1.postApiClReopen().request(link, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(link),
      })
    },
  })
}