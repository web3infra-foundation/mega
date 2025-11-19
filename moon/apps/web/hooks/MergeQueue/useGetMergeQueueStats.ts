import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueStatsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiMergeQueueStats()

/**
 * Hook to fetch statistics about the merge queue.
 *
 * @param params - Optional request parameters.
 * @returns A query object containing merge queue statistics including counts for
 * waiting, testing, merging, merged, and failed items.
 * The query automatically refetches every 10 seconds, even in the background.
 *
 * @example
 * ```tsx
 * const { data } = useGetMergeQueueStats()
 * const stats = data?.data?.stats
 * console.log(`Total items: ${stats?.total_items}`)
 * ```
 */
export function useGetMergeQueueStats(params?: RequestParams) {
  return useQuery<GetApiMergeQueueStatsData>({
    queryKey: [...query.requestKey(), params],
    queryFn: () => query.request(params),
    refetchInterval: 10000,
    refetchIntervalInBackground: true
  })
}
