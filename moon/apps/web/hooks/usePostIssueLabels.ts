import { useMutation } from '@tanstack/react-query'

import { LabelUpdatePayload, PostApiIssueLabelsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueLabels() {
  return useMutation<PostApiIssueLabelsData, Error, { data: LabelUpdatePayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiIssueLabels().request(data, params)
  })
}
