import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiBlameParams, RequestParams } from '@gitmono/types'

export function useGetBlame(params: GetApiBlameParams, requestParams?: RequestParams) {
  return useQuery({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiBlame().requestKey(params),
    queryFn: () => legacyApiClient.v1.getApiBlame().request(params, requestParams),
    enabled: !!params.path,
  })
}

