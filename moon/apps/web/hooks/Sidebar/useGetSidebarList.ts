import { useQuery } from '@tanstack/react-query'

import { GetApiSidebarListData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetSidebarList(params?: RequestParams) {
  return useQuery<GetApiSidebarListData, Error>({
    queryKey: ['sidebar', 'list', params],
    queryFn: () => legacyApiClient.v1.getApiSidebarList().request(params),
    staleTime: 10 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false
  })
}
