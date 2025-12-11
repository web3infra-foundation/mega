import { useQuery } from '@tanstack/react-query'

import {
  Pagination,
  PostApiCommitsFilesChangedData,
  PostApiCommitsFilesChangedParams,
  RequestParams
} from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostCommitsFilesChanged(
  params: PostApiCommitsFilesChangedParams,
  data: Pagination,
  requestParams?: RequestParams
) {
  return useQuery<PostApiCommitsFilesChangedData>({
    queryKey: [params, data, requestParams],
    queryFn: () => legacyApiClient.v1.postApiCommitsFilesChanged().request(params, data, requestParams),
    enabled: Boolean(params.sha && params.path)
  })
}
