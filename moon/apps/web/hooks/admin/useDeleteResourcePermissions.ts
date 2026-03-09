import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { DeleteApiAdminResourcesPermissionsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useDeleteResourcePermissions() {
  const queryClient = useQueryClient()

  return useMutation<
    DeleteApiAdminResourcesPermissionsData,
    Error,
    { resourceType: string; resourceId: string; params?: RequestParams }
  >({
    mutationFn: async ({ resourceType, resourceId, params }) => {
      const response = await legacyApiClient.v1
        .deleteApiAdminResourcesPermissions()
        .request(resourceType, resourceId, params)

      if (response && typeof response === 'object' && 'req_result' in response && !response.req_result) {
        throw new Error(response.err_message || 'Delete failed')
      }

      return response
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiAdminResourcesPermissions().baseKey
      })
    }
  })
}
