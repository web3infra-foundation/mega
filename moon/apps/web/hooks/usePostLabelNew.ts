import { useMutation } from '@tanstack/react-query'

import { NewLabel, PostApiLabelNewData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostLabelNew() {
  return useMutation<PostApiLabelNewData, Error, { data: NewLabel; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiLabelNew().request(data, params)
  })
}
