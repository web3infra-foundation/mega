import { useQuery } from '@tanstack/react-query'

import { GetApiCommitsMuiTreeData, GetApiCommitsMuiTreeParams, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetCommitsMuiTree(params: GetApiCommitsMuiTreeParams, requestParams?: RequestParams) {
  return useQuery<GetApiCommitsMuiTreeData>({
    queryKey: [params, requestParams],
    queryFn: () => legacyApiClient.v1.getApiCommitsMuiTree().request(params, requestParams),
    enabled: Boolean(params.sha && params.path)
  })
}
