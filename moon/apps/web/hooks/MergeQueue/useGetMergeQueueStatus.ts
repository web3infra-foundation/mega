import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueStatusByClLinkData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

interface UseGetMergeQueueStatusOptions {
  enabled?: boolean
  refetchInterval?: number | false
}

/**
 * Hook to check if a specific change list (CL) is in the merge queue.
 *
 * @param clLink - The CL link to check the queue status for.
 * @param params - Optional request parameters.
 * @param options - Query behaviour overrides such as polling controls.
 * @returns A query object containing the queue status, including whether the CL is in the queue
 * and its queue item details if present.
 *
 * @example
 * ```tsx
 * const { data } = useGetMergeQueueStatus('cl/123', undefined, { enabled: true, refetchInterval: 3000 })
 * const inQueue = data?.data?.in_queue
 * const queueItem = data?.data?.item
 * ```
 */
export function useGetMergeQueueStatus(
  clLink: string,
  params?: RequestParams,
  options: UseGetMergeQueueStatusOptions = {}
) {
  const query = legacyApiClient.v1.getApiMergeQueueStatusByClLink()

  return useQuery<GetApiMergeQueueStatusByClLinkData>({
    queryKey: [...query.requestKey(clLink), params],
    queryFn: () => query.request(clLink, params),
    enabled: !!clLink && (options.enabled ?? false),
    refetchInterval: options.refetchInterval ?? false
  })
}
