import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiMergeQueueList()

/**
 * Hook to fetch the list of items in the merge queue.
 *
 * @param params - Optional request parameters for filtering or pagination.
 * @returns A query object containing the merge queue list data.
 * The query automatically refetches every 10 seconds.
 *
 * @example
 * ```tsx
 * const { data, isLoading } = useGetMergeQueueList()
 * const items = data?.data?.items || []
 * ```
 */
export function useGetMergeQueueList(params?: RequestParams) {
  return useQuery<GetApiMergeQueueListData>({
    queryKey: [...query.requestKey(), params],
    queryFn: () => query.request(params),
    refetchInterval: 10000
  })
}
