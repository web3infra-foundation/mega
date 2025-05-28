// hooks/usePostApiMrList.ts
import { useMutation } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { PageParamsMRStatusParams, RequestParams, PostApiMrListData } from '@gitmono/types'

export function usePostMrList() {
  return useMutation<PostApiMrListData, Error, { data: PageParamsMRStatusParams; params?: RequestParams }>({
    mutationFn: ({ data, params }) =>
      legacyApiClient.v1.postApiMrList().request(data, params),
  })
}