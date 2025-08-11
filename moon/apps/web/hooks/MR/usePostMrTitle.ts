import { useMutation } from '@tanstack/react-query'

import { ContentPayload, PostApiMrTitleData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostMrTitle() {
  return useMutation<PostApiMrTitleData, Error, { link: string; data: ContentPayload; params?: RequestParams }>({
    mutationFn: ({ link, data, params }) => legacyApiClient.v1.postApiMrTitle().request(link, data, params)
  })
}
