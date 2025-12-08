import { useMutation } from '@tanstack/react-query'

import { GetApiCommitsDetailData, GetApiCommitsDetailParams, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function useGetCommitsDetail() {
  return useMutation<GetApiCommitsDetailData, Error, { data: GetApiCommitsDetailParams; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.getApiCommitsDetail().request(data, params)
  })
}
