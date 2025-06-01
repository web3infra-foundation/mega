import { PostApiIssueCommentData, RequestParams, SaveCommentRequest } from '@gitmono/types/generated'
import { useMutation } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueComment() {
  return useMutation<
    PostApiIssueCommentData,
    Error,
    { link: string; data: SaveCommentRequest; params?: RequestParams }
  >({
    mutationFn: ({ link, data, params }) => legacyApiClient.v1.postApiIssueComment().request(link, data, params)
  })
}
