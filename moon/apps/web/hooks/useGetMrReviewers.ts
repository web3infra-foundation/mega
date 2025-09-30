import { useQuery } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import { GetApiMrReviewersData, ReviewerInfo } from "@gitmono/types";

export const useGetMrReviewers = (link: string): {
  reviewers: ReviewerInfo[],
  isLoading: boolean,
  refetch: () => void
} => {
  const { data, isLoading, refetch } = useQuery<GetApiMrReviewersData>({
    queryKey: legacyApiClient.v1.getApiMrReviewers().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.getApiMrReviewers().request(link)
    }
  })

  return { reviewers: data?.data?.result ?? [], isLoading, refetch }
}