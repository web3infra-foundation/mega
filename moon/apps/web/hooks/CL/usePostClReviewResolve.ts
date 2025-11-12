import { useMutation } from '@tanstack/react-query'

import { ChangeReviewStatePayload, PostApiClReviewResolveData } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export const usePostClReviewResolve = () => {
  return useMutation<PostApiClReviewResolveData, Error, { link: string; data: ChangeReviewStatePayload }>({
    mutationFn: ({ link, data }) => legacyApiClient.v1.postApiClReviewResolve().request(link, data)
  })
}
