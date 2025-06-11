import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiTreeParams, RequestParams } from '@gitmono/types'

export function useGetTree(params: GetApiTreeParams, requestParams?: RequestParams) {
  return useQuery({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiTree().requestKey(params),
    queryFn: () => legacyApiClient.v1.getApiTree().request(params, requestParams),
    enabled: !!params.path,
  })
}