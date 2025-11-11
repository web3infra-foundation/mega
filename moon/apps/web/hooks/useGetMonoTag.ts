import { useQuery } from '@tanstack/react-query'

import { GetApiTagsByNameData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

const getTag = legacyApiClient.v1.getApiTagsByName()

export function useGetMonoTag(name: string, params?: RequestParams, enabled: boolean = true) {
  return useQuery<GetApiTagsByNameData>({
    queryKey: [getTag.requestKey(name), params],
    queryFn: () => getTag.request(name, params),
    enabled,
    staleTime: 60_000
  })
}
