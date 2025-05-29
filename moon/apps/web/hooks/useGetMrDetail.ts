import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { RequestParams } from '@gitmono/types'

export function useGetMrDetail(id: string, params?: RequestParams) {
  return useQuery({
    queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(id),
    queryFn: () => legacyApiClient.v1.getApiMrDetail().request(id, params),
    enabled: !!id, 
  })
}