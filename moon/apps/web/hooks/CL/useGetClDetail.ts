import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { RequestParams } from '@gitmono/types'

export function useGetClDetail(id: string, params?: RequestParams) {
  return useQuery({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.getApiClDetail().requestKey(id),
    queryFn: () => legacyApiClient.v1.getApiClDetail().request(id, params),
    enabled: !!id,
  })
}