import { useQuery } from '@tanstack/react-query'

import { GetApiClReviewersData, ReviewerInfo } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

export const useGetClReviewers = (
  link: string
): {
  reviewers: ReviewerInfo[]
  isLoading: boolean
  refetch: () => void
} => {
  const { data, isLoading, refetch } = useQuery<GetApiClReviewersData>({
    queryKey: legacyApiClient.v1.getApiClReviewers().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.getApiClReviewers().request(link)
    }
  })

  return { reviewers: data?.data?.result ?? [], isLoading, refetch }
}
