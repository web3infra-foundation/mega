import { useMutation } from '@tanstack/react-query'

import { LabelUpdatePayload, PostApiClLabelsData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostCLLabels() {
  return useMutation<PostApiClLabelsData, Error, { data: LabelUpdatePayload; params?: RequestParams }>({
    mutationFn: ({ data, params }) => legacyApiClient.v1.postApiClLabels().request(data, params)
  })
}
