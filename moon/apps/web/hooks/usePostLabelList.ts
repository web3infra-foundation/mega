// hooks/usePostLabelList.ts
import { PageParamsString, PostApiLabelListData, RequestParams } from '@gitmono/types'
import { legacyApiClient } from '@/utils/queryClient'
import { useMutation } from '@tanstack/react-query'

export function usePostLabelList() {
  return useMutation<PostApiLabelListData, Error, { data: PageParamsString, params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiLabelList().request(data, params)
  })
}