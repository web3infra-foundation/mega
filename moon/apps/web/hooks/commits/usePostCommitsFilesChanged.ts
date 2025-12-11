import { useQuery } from '@tanstack/react-query'

import { Pagination, PostApiCommitsFilesChangedData, PostApiCommitsFilesChangedParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostCommitsFilesChanged(params: PostApiCommitsFilesChangedParams | null, data: Pagination) {
  return useQuery<PostApiCommitsFilesChangedData>({
    queryKey: ['commits-files-changed', params, data],
    queryFn: () => legacyApiClient.v1.postApiCommitsFilesChanged().request(params!, data),
    enabled: Boolean(params?.sha && params?.path)
  })
}
