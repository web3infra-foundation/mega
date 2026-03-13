import { useQuery } from '@tanstack/react-query'

import type { GetApiAdminUsersPermissionsByResourceIdData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

interface UseGetUserPermissionsOptions extends RequestParams {
  enabled?: boolean
}

export function useGetUserPermissions(
  username: string,
  resourceType: string,
  resourceId: string,
  options?: UseGetUserPermissionsOptions
) {
  const { enabled = true, ...params } = options || {}

  return useQuery<GetApiAdminUsersPermissionsByResourceIdData, Error>({
    queryKey: [
      ...legacyApiClient.v1.getApiAdminUsersPermissionsByResourceId().requestKey(username, resourceType, resourceId),
      params
    ],
    queryFn: () =>
      legacyApiClient.v1.getApiAdminUsersPermissionsByResourceId().request(username, resourceType, resourceId, params),
    enabled: enabled && !!username && !!resourceType && !!resourceId,
    staleTime: 0,
    retry: false
  })
}
