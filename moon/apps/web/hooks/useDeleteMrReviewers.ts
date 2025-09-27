import { useQuery } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";
import { DeleteApiMrReviewersData } from "@gitmono/types";

export const useDeleteMrReviewers = (
  link: string,
  usernames: string[]
): {
  data: string,
  isLoading: boolean,
} => {
  const { data, isLoading } = useQuery<DeleteApiMrReviewersData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.deleteApiMrReviewers().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.deleteApiMrReviewers().request(
        link,
        {
          reviewer_usernames: usernames
        }
      )
    }
  })

  return { data: data?.data?? "", isLoading }
}