import { useMutation, useQueryClient } from '@tanstack/react-query'

import type {
  InitializeCommentRequest,
  PostApiCodeReviewCommentInitData,
  RequestParams
} from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

const initCommentMutation = legacyApiClient.v1.postApiCodeReviewCommentInit()

/**
 * 在指定行创建新的评论 Thread
 * POST /code_review/{link}/comment/init
 */
export function useInitComment() {
  const queryClient = useQueryClient()

  return useMutation<
    PostApiCodeReviewCommentInitData,
    Error,
    { link: string; data: InitializeCommentRequest; params?: RequestParams }
  >({
    mutationFn: ({ link, data, params }) => initCommentMutation.request(link, data, params),

    onSuccess: (_response, { link }) => {
      const queryKey = legacyApiClient.v1.getApiCodeReviewComments().requestKey(link)

      queryClient.refetchQueries({ queryKey, type: 'active' })
    }
  })
}
