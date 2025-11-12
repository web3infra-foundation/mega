import { useMutation, useQueryClient } from '@tanstack/react-query'

import { DeleteApiClReviewersData, ReviewerPayload } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export const useDeleteClReviewers = () => {
  const queryClient = useQueryClient()

  return useMutation<DeleteApiClReviewersData, Error, { link: string; data: ReviewerPayload }>({
    mutationFn: ({ link, data }) => legacyApiClient.v1.deleteApiClReviewers().request(link, data),
    onSuccess: (_, { link }) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClReviewers().requestKey(link)
      })
    }
  })
}
