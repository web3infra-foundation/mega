import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueStatsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiMergeQueueStats()

export function useGetMergeQueueStats(params?: RequestParams) {
  return useQuery<GetApiMergeQueueStatsData>({
    queryKey: [...query.requestKey(), params],
    queryFn: () => query.request(params),
    refetchInterval: 10000,
    refetchIntervalInBackground: true
  })
}
