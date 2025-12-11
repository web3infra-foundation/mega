import { useMutation } from '@tanstack/react-query'

import { DeleteApiSidebarByIdData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useDeleteSidebarById() {
  return useMutation<DeleteApiSidebarByIdData, Error, { id: number; params?: RequestParams }>({
    mutationFn: ({ id, params }) => legacyApiClient.v1.deleteApiSidebarById().request(id, params)
  })
}
