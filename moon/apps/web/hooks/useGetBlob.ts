import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiBlobParams, RequestParams } from '@gitmono/types'

export function useGetBlob(params: GetApiBlobParams & { refs?: string }, requestParams?: RequestParams) {
  const finalParams: any = { path: params.path }
  
  if (params.refs) finalParams.refs = params.refs

  return useQuery({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiBlob().requestKey(finalParams as GetApiBlobParams),
    queryFn: () => legacyApiClient.v1.getApiBlob().request(finalParams, requestParams),
    enabled: !!params.path,
  })
}