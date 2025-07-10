// hooks/usePostApiMrList.ts
import { useMutation } from '@tanstack/react-query'

import { PageParamsListPayload, PostApiMrListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostMrList() {
  return useMutation<PostApiMrListData, Error, { data: PageParamsListPayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiMrList().request(data, params)
  })
}
