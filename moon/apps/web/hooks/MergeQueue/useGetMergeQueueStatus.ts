import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueStatusByClLinkData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

/**
 * Hook to check if a specific change list (CL) is in the merge queue.
 *
 * @param clLink - The CL link to check the queue status for.
 * @param params - Optional request parameters.
 * @returns A query object containing the queue status, including whether the CL is in the queue
 * and its queue item details if present. The query is only enabled when clLink is provided
 * and automatically refetches every 10 seconds, even in the background.
 *
 * @example
 * ```tsx
 * const { data } = useGetMergeQueueStatus('cl/123')
 * const inQueue = data?.data?.in_queue
 * const queueItem = data?.data?.item
 * ```
 */
export function useGetMergeQueueStatus(clLink: string, params?: RequestParams) {
  const query = legacyApiClient.v1.getApiMergeQueueStatusByClLink()

  return useQuery<GetApiMergeQueueStatusByClLinkData>({
    queryKey: [...query.requestKey(clLink), params],
    queryFn: () => query.request(clLink, params),
    enabled: !!clLink,
    refetchInterval: 10000,
    refetchIntervalInBackground: true
  })
}
