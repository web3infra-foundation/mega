import { useMutation } from '@tanstack/react-query'

import { PageParamsCommitHistoryParams, PostApiCommitsHistoryData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetCommitsHistory() {
  return useMutation<PostApiCommitsHistoryData, Error, { data: PageParamsCommitHistoryParams; params?: RequestParams }>(
    {
      mutationFn: ({ data, params }) => legacyApiClient.v1.postApiCommitsHistory().request(data, params)
    }
  )
}
