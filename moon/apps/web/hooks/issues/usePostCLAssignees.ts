import { useMutation } from '@tanstack/react-query'

import {AssigneeUpdatePayload,  PostApiClAssigneesData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostCLAssignees() {
  return useMutation<PostApiClAssigneesData, Error, { data: AssigneeUpdatePayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiClAssignees().request(data, params)
  })
}
