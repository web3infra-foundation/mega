import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { PutApiAdminResourcesPermissionsData, RequestParams, SetPermissionsRequest } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

interface UpdateResourcePermissionsParams {
  resourceType: string
  resourceId: string
  data: SetPermissionsRequest
  params?: RequestParams
}

export function useUpdateResourcePermissions() {
  const queryClient = useQueryClient()

  return useMutation<PutApiAdminResourcesPermissionsData, Error, UpdateResourcePermissionsParams>({
    mutationFn: async ({ resourceType, resourceId, data, params }) => {
      const response = await legacyApiClient.v1
        .putApiAdminResourcesPermissions()
        .request(resourceType, resourceId, data, params)

      if (response && typeof response === 'object' && 'req_result' in response && !response.req_result) {
        throw new Error(response.err_message || 'Update permissions failed')
      }

      return response
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1
          .getApiAdminResourcesPermissions()
          .requestKey(variables.resourceType, variables.resourceId)
      })

      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiAdminResourcesPermissions().baseKey
      })
    }
  })
}
