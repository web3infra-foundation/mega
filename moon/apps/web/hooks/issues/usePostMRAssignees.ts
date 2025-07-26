import { useMutation } from '@tanstack/react-query'

import {AssigneeUpdatePayload,  PostApiMrAssigneesData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostMRAssignees() {
  return useMutation<PostApiMrAssigneesData, Error, { data: AssigneeUpdatePayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiMrAssignees().request(data, params)
  })
}
