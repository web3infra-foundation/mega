import { useQuery } from '@tanstack/react-query'

import type { GetOrionClientStatusByIdData, RequestParams } from '@gitmono/types/generated'

import { orionApiClient } from '@/utils/queryClient'

export function useGetOrionClientStatusById(id: string, params?: RequestParams, refetchIntervalMs?: number) {
  return useQuery<GetOrionClientStatusByIdData, Error>({
    queryKey: [...orionApiClient.id.getOrionClientStatusById().requestKey(id), params],
    queryFn: () => orionApiClient.id.getOrionClientStatusById().request(id, params),
    refetchInterval: refetchIntervalMs,
    refetchIntervalInBackground: Boolean(refetchIntervalMs),
    enabled: Boolean(id)
  })
}

export const fetchOrionClientStatusById = (id: string, params?: RequestParams) => {
  return orionApiClient.id.getOrionClientStatusById().request(id, params)
}
