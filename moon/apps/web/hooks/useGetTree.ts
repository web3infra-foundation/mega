import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiTreeParams, RequestParams } from '@gitmono/types'

export function useGetTree(params: GetApiTreeParams, requestParams?: RequestParams) {
  const finalParams: GetApiTreeParams = { path: params.path }
  
  if ((params as any).refs) (finalParams as any).refs = (params as any).refs

  return useQuery({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiTree().requestKey(finalParams),
    queryFn: () => legacyApiClient.v1.getApiTree().request(finalParams, requestParams),
    enabled: !!finalParams.path,
  })
}