import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const query = legacyApiClient.v1.getApiMergeQueueList()

export function useGetMergeQueueList(params?: RequestParams) {
  return useQuery<GetApiMergeQueueListData>({
    queryKey: [...query.requestKey(), params],
    queryFn: () => query.request(params),
    refetchInterval: 10000
  })
}
