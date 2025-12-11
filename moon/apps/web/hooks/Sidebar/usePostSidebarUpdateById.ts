import { useMutation } from '@tanstack/react-query'

import { PostApiSidebarUpdateByIdData, RequestParams, UpdateSidebarPayload } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostSidebarUpdateById() {
  return useMutation<
    PostApiSidebarUpdateByIdData,
    Error,
    { id: number; data: UpdateSidebarPayload; params?: RequestParams }
  >({
    mutationFn: ({ id, data, params }) => legacyApiClient.v1.postApiSidebarUpdateById().request(id, data, params)
  })
}
