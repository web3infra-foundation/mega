import { useMutation } from '@tanstack/react-query'

import { PageParamsListPayload, PostApiIssueListData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetIssueLists() {
  return useMutation<PostApiIssueListData, Error, { data: PageParamsListPayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiIssueList().request(data, params)
  })
}
