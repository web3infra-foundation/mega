import { useMutation } from '@tanstack/react-query'

import {
  Pagination,
  PostApiCommitsFilesChangedData,
  PostApiCommitsFilesChangedParams,
  RequestParams
} from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetCommitsDetail() {
  return useMutation<
    PostApiCommitsFilesChangedData,
    Error,
    { params: PostApiCommitsFilesChangedParams; data: Pagination; requestParams?: RequestParams }
  >({
    mutationFn: ({ params, data, requestParams }) =>
      legacyApiClient.v1.postApiCommitsFilesChanged().request(params, data, requestParams)
  })
}
