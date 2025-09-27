import { useQuery } from "@tanstack/react-query";
import { PostApiMrReviewersData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewers = (
  link: string,
  user_names: string[]
) => {
  return useQuery<PostApiMrReviewersData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.postApiMrReviewers().requestKey(link),
    queryFn: () =>
      legacyApiClient.v1.postApiMrReviewers().request(link,{reviewer_usernames: user_names})
  })
};