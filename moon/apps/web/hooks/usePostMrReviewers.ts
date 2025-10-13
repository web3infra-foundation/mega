import { useMutation, useQueryClient } from "@tanstack/react-query";
import { PostApiMrReviewersData, ReviewerPayload } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewers = (
) => {
  const queryClient = useQueryClient()

  return useMutation<PostApiMrReviewersData, Error, {link: string, data: ReviewerPayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.postApiMrReviewers().request(link, data),
    onSuccess: (_, { link }) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrReviewers().requestKey(link)
      })
    }
  })
};
