import { useQuery } from "@tanstack/react-query";
import { PostApiMrReviewResolveData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewResolve = (
  link: string,
  new_state: boolean,
  review_id: number
): {
  data: string,
  isLoading: boolean
} => {
  const { data, isLoading } = useQuery<PostApiMrReviewResolveData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.postApiMrReviewResolve().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.postApiMrReviewResolve().request(
        link,
        {
          new_state,
          review_id
        }
      )
    }
  })

  return {
    data: data?.data ?? "",
    isLoading
  }
}