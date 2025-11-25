import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiMergeQueueList()

interface UseGetMergeQueueListOptions {
  refetchInterval?: number | false
}

/**
 * Hook to fetch the list of items in the merge queue.
 *
 * @param params - Optional request parameters for filtering or pagination.
 * @param options - Query behaviour overrides such as polling controls.
 * @returns A query object containing the merge queue list data.
 *
 * @example
 * ```tsx
 * const { data, isLoading } = useGetMergeQueueList(undefined, { refetchInterval: 3000 })
 * const items = data?.data?.items || []
 * ```
 */
export function useGetMergeQueueList(params?: RequestParams, options: UseGetMergeQueueListOptions = {}) {
  return useQuery<GetApiMergeQueueListData>({
    queryKey: [...query.requestKey(), params],
    queryFn: () => query.request(params),
    refetchInterval: options.refetchInterval ?? false
  })
}
