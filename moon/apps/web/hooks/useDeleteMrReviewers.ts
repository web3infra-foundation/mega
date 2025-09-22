import { useQuery } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";
import { DeleteApiMrRemoveReviewersData } from "@gitmono/types";

export const useDeleteMrReviewers = (
  link: string,
  usernames: string[]
): {
  data: string,
  isLoading: boolean,
} => {
  const { data, isLoading } = useQuery<DeleteApiMrRemoveReviewersData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.deleteApiMrRemoveReviewers().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.deleteApiMrRemoveReviewers().request(
        link,
        {
          reviewer_usernames: usernames
        }
      )
    }
  })

  return { data: data?.data?? "", isLoading }
}