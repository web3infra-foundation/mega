import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { SaveCommentRequest, PostApiMrCommentData, RequestParams } from '@gitmono/types'

export function usePostMrComment(link: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<PostApiMrCommentData, Error, SaveCommentRequest>({
    mutationKey: legacyApiClient.v1.postApiMrComment().requestKey(link),
    mutationFn: (data) =>
      legacyApiClient.v1.postApiMrComment().request(link, data, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(link),
      })
    },
  })
}