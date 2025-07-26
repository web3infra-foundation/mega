import { useQuery } from '@tanstack/react-query'

import { CommonResultIssueDetailRes, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetIssueDetail(id: string, params?: RequestParams) {
  return useQuery<CommonResultIssueDetailRes, Error>({
    queryKey: [legacyApiClient.v1.getApiIssueDetail().requestKey(id), params],
    queryFn: () => legacyApiClient.v1.getApiIssueDetail().request(id, params),

    enabled: !!id
  })
}
