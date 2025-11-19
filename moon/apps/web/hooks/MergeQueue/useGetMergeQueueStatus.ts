import { useQuery } from '@tanstack/react-query'

import type { GetApiMergeQueueStatusByClLinkData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

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
