// hooks/usePostLabelList.ts
import { useMutation } from '@tanstack/react-query'

import { PageParamsString, PostApiLabelListData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostLabelList() {
  return useMutation<PostApiLabelListData, Error, { data: PageParamsString; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiLabelList().request(data, params)
  })
}
