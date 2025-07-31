import { LabelUpdatePayload, PostApiMrLabelsData, RequestParams } from '@gitmono/types'
import { useMutation } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'

export function usePostMRLabels() {
  return useMutation<PostApiMrLabelsData, Error, {data: LabelUpdatePayload, params?: RequestParams}>({
    mutationFn: ({data, params}) => legacyApiClient.v1.postApiMrLabels().request(data,params)
  })
}