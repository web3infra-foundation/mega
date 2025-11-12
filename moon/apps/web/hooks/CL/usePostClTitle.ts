import { useMutation } from '@tanstack/react-query'

import { ContentPayload, PostApiClTitleData, RequestParams } from '@gitmono/types/generated'

import { legacyApiClient } from '@/utils/queryClient'

export function usePostClTitle() {
  return useMutation<PostApiClTitleData, Error, { link: string; data: ContentPayload; params?: RequestParams }>({
    mutationFn: ({ link, data, params }) => legacyApiClient.v1.postApiClTitle().request(link, data, params)
  })
}
