import { useQuery } from '@tanstack/react-query'

import { GetApiCommitsMuiTreeData, GetApiCommitsMuiTreeParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetCommitsMuiTree(params: GetApiCommitsMuiTreeParams | null) {
  return useQuery<GetApiCommitsMuiTreeData>({
    queryKey: ['commits-mui-tree', params],
    queryFn: () => legacyApiClient.v1.getApiCommitsMuiTree().request(params!),
    enabled: Boolean(params?.sha && params?.path)
  })
}
