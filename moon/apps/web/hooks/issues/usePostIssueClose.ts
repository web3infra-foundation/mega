import { PostApiIssueCloseData, RequestParams } from '@gitmono/types/generated'
import { useMutation } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueClose() {
  return useMutation<PostApiIssueCloseData, Error, { link: string; params?: RequestParams }>({
    mutationFn: ({ link, params }) => legacyApiClient.v1.postApiIssueClose().request(link, params)
  })
}
