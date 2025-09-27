import { useQuery } from "@tanstack/react-query";
import { PostApiMrReviewerApproveData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewerApprove = (
  link: string,
  approved: boolean
): {
  data: string,
  isLoading: boolean
} => {
  const { data, isLoading } = useQuery<PostApiMrReviewerApproveData>({
    // eslint-disable-next-line @tanstack/query/exhaustive-deps
    queryKey: legacyApiClient.v1.postApiMrReviewerApprove().requestKey(link),
    queryFn: async () => {
      return await legacyApiClient.v1.postApiMrReviewerApprove().request(
        link,
        {
          approved
        }
      )
    }
  })

  return {
    data: data?.data ?? "",
    isLoading
  }
}