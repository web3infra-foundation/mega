import { useQuery } from '@tanstack/react-query'

import type { GetApiAdminListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useAdminList(params?: RequestParams) {
  return useQuery<GetApiAdminListData, Error>({
    queryKey: [...legacyApiClient.v1.getApiAdminList().requestKey(), params],
    queryFn: () => legacyApiClient.v1.getApiAdminList().request(params),
    staleTime: 0,
    retry: false
  })
}
