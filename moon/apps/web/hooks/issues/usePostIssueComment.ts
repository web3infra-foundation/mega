import { useMutation } from '@tanstack/react-query'

import { ContentPayload, PostApiIssueCommentData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueComment() {
  return useMutation<PostApiIssueCommentData, Error, { link: string; data: ContentPayload; params?: RequestParams }>({
    mutationFn: ({ link, data, params }) => legacyApiClient.v1.postApiIssueComment().request(link, data, params)
  })
}
