import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiBlobParams, RequestParams } from '@gitmono/types'

export function useGetBlob(params: GetApiBlobParams, requestParams?: RequestParams) {
  return useQuery({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiBlob().requestKey(params),
    queryFn: () => legacyApiClient.v1.getApiBlob().request(params, requestParams),
    enabled: !!params.path,
  })
}