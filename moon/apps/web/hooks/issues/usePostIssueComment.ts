import { PostApiIssueCommentData, RequestParams, ContentPayload } from '@gitmono/types/generated'
import { useMutation } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueComment() {
  return useMutation<
    PostApiIssueCommentData,
    Error,
    { link: string; data: ContentPayload; params?: RequestParams }
  >({
    mutationFn: ({ link, data, params }) => legacyApiClient.v1.postApiIssueComment().request(link, data, params)
  })
}
