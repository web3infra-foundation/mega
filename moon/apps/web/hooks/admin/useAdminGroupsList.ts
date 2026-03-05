import { useQuery } from '@tanstack/react-query'

import type { PageParamsEmptyListAdditional, PostApiAdminGroupsListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useAdminGroupsList(data: PageParamsEmptyListAdditional, params?: RequestParams) {
  return useQuery<PostApiAdminGroupsListData, Error>({
    queryKey: [...legacyApiClient.v1.postApiAdminGroupsList().requestKey(), data, params],
    queryFn: () => legacyApiClient.v1.postApiAdminGroupsList().request(data, params),
    staleTime: 0,
    retry: false
  })
}
