import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { ContentPayload, PostApiClCommentData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClComment(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiClCommentData, Error, ContentPayload>({
    mutationKey: legacyApiClient.v1.postApiClComment().requestKey(link),
    mutationFn: (data) => legacyApiClient.v1.postApiClComment().request(link, data, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(link)
      })
    }
  })
}
