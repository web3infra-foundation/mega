import { useMutation } from '@tanstack/react-query'

import { PostApiSidebarSyncData, PostApiSidebarSyncPayload, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostSidebarSync() {
  return useMutation<PostApiSidebarSyncData, Error, { data: PostApiSidebarSyncPayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiSidebarSync().request(data, params)
  })
}
