import { useMutation, useQueryClient } from '@tanstack/react-query'

import { PostApiClReviewersData, ReviewerPayload } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export const usePostClReviewers = () => {
  const queryClient = useQueryClient()

  return useMutation<PostApiClReviewersData, Error, { link: string; data: ReviewerPayload }>({
    mutationFn: ({ link, data }) => legacyApiClient.v1.postApiClReviewers().request(link, data),
    onSuccess: (_, { link }) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClReviewers().requestKey(link)
      })
    }
  })
}
