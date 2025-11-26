import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueStatsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiMergeQueueStats()

interface UseGetMergeQueueStatsOptions {
  refetchInterval?: number | false
}

/**
 * Hook to fetch statistics about the merge queue.
 *
 * @param params - Optional request parameters.
 * @param options - Query behaviour overrides such as polling controls.
 * @returns A query object containing merge queue statistics including counts for
 * waiting, testing, merging, merged, and failed items.
 *
 * @example
 * ```tsx
 * const { data } = useGetMergeQueueStats(undefined, { refetchInterval: 3000 })
 * const stats = data?.data?.stats
 * console.log(`Total items: ${stats?.total_items}`)
 * ```
 */
export function useGetMergeQueueStats(params?: RequestParams, options: UseGetMergeQueueStatsOptions = {}) {
  return useQuery<GetApiMergeQueueStatsData>({
    queryKey: [...query.requestKey(), params],
    queryFn: () => query.request(params),
    refetchInterval: options.refetchInterval ?? false
  })
}
