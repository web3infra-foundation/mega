import { useMutation } from '@tanstack/react-query'

import { PageParamsListPayload, PostApiClListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClList() {
  return useMutation<PostApiClListData, Error, { data: PageParamsListPayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiClList().request(data, params)
  })
}
