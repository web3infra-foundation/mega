import { useQuery } from "@tanstack/react-query";
import { PostApiMrReviewResolveData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewerNewState = (
  link: string,
  state: boolean
): {
  data: string,
  isLoading: boolean
} => {
  const { data, isLoading } = useQuery<PostApiMrReviewResolveData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.postApiMrReviewerNewState().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.postApiMrReviewerNewState().request(
        link,
        {
          state
        }
      )
    }
  })

  return {
    data: data?.data ?? "",
    isLoading
  }
}