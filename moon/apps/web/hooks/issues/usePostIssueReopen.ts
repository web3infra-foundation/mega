import { PostApiIssueReopenData, RequestParams } from '@gitmono/types/generated'
import { useMutation } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueReopen() {
  return useMutation<PostApiIssueReopenData, Error, { link: string; params?: RequestParams }>({
    mutationFn: ({ link, params }) => legacyApiClient.v1.postApiIssueReopen().request(link, params)
  })
}
