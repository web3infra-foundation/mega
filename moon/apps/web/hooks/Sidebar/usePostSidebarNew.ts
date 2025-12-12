import { useMutation } from '@tanstack/react-query'

import { CreateSidebarPayload, PostApiSidebarNewData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostSidebarNew() {
  return useMutation<PostApiSidebarNewData, Error, { data: CreateSidebarPayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiSidebarNew().request(data, params)
  })
}
