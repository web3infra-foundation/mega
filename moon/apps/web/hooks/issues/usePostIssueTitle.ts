import { useMutation } from '@tanstack/react-query'

import { ContentPayload, PostApiIssueTitleData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueTitle() {
  return useMutation<PostApiIssueTitleData, Error, { link: string; data: ContentPayload; params?: RequestParams }>({
    mutationFn: ({ link, data, params }) => legacyApiClient.v1.postApiIssueTitle().request(link, data, params)
  })
}
