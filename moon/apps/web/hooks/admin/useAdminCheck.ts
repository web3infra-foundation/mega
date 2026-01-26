import { useQuery } from '@tanstack/react-query'

import type { GetApiAdminMeData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useAdminCheck(params?: RequestParams) {
  return useQuery<GetApiAdminMeData, Error>({
    queryKey: [...legacyApiClient.v1.getApiAdminMe().requestKey(), params],
    queryFn: () => legacyApiClient.v1.getApiAdminMe().request(params),
    staleTime: 0,
    retry: false
  })
}
