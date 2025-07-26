import { useMutation } from '@tanstack/react-query'

import { AssigneeUpdatePayload, PostApiIssueAssigneesData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostIssueAssignees() {
  return useMutation<PostApiIssueAssigneesData, Error, { data: AssigneeUpdatePayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiIssueAssignees().request(data, params)
  })
}
