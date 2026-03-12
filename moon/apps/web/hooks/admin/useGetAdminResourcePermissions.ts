import { useQuery } from '@tanstack/react-query'

import type { GetApiAdminResourcesPermissionsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

interface UseGetAdminResourcePermissionsOptions extends RequestParams {
  enabled?: boolean
}

export function useGetAdminResourcePermissions(
  resourceType: string,
  resourceId: string,
  options?: UseGetAdminResourcePermissionsOptions
) {
  const { enabled = true, ...params } = options || {}

  return useQuery<GetApiAdminResourcesPermissionsData, Error>({
    queryKey: [...legacyApiClient.v1.getApiAdminResourcesPermissions().requestKey(resourceType, resourceId), params],
    queryFn: () => legacyApiClient.v1.getApiAdminResourcesPermissions().request(resourceType, resourceId, params),
    enabled: enabled && !!resourceType && !!resourceId,
    staleTime: 0,
    retry: false
  })
}
