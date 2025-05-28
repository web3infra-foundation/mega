import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { RequestParams } from '@gitmono/types'
import type { GetApiTreePathCanCloneData, GetApiTreePathCanCloneParams } from '@gitmono/types/generated' // 按你的路径调整

export function useGetTreePathCanClone(params: GetApiTreePathCanCloneParams, requestParams?: RequestParams) {
  return useQuery<GetApiTreePathCanCloneData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiTreePathCanClone().requestKey(params),
    queryFn: () => 
      legacyApiClient.v1.getApiTreePathCanClone().request(params, requestParams),
    enabled: !!params.path, 
  })
}