// hooks/usePostApiMrList.ts
import { useMutation } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { PageParamsListPayload, RequestParams, PostApiMrListData } from '@gitmono/types'

export function usePostMrList() {
  return useMutation<PostApiMrListData, Error, { data: PageParamsListPayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) =>
      legacyApiClient.v1.postApiMrList().request(data, params),
  })
}