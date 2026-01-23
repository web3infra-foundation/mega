import { useQuery } from '@tanstack/react-query'

import { GetApiClUpdateStatusData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Get CL update status
 * Check if the current CL branch needs to be updated (whether it's behind the target branch)
 *
 * @param link - Unique identifier of the CL
 * @param enabled - Whether to enable the query, defaults to true
 * @param refetchInterval - Auto-refresh interval in milliseconds, no auto-refresh by default
 */
export const useGetClUpdateStatus = (link: string, enabled: boolean = true, refetchInterval?: number) => {
  const query = legacyApiClient.v1.getApiClUpdateStatus()

  return useQuery<GetApiClUpdateStatusData>({
    queryKey: query.requestKey(link),
    queryFn: () => query.request(link),
    enabled: enabled && !!link,
    refetchInterval,
    staleTime: 30000 // Consider data fresh within 30 seconds
  })
}
