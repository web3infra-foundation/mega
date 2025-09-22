import { useQuery } from "@tanstack/react-query";
import { PostApiMrAddReviewersData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewers = (
  link: string,
  user_names: string[]
) => {
  return useQuery<PostApiMrAddReviewersData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.postApiMrAddReviewers().requestKey(link),
    queryFn: () =>
      legacyApiClient.v1.postApiMrAddReviewers().request(link,{reviewer_usernames: user_names})
  })
};