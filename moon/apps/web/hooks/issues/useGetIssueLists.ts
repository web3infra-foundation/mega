import { PageParamsStatusParams, PostApiIssueListData, RequestParams } from '@gitmono/types/generated'
import { useMutation } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetIssueLists() {
  return useMutation<PostApiIssueListData, Error, { data: PageParamsStatusParams; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiIssueList().request(data, params)
  })
}
