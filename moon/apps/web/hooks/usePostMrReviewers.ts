import { useMutation } from "@tanstack/react-query";
import { PostApiMrReviewersData, ReviewerPayload } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewers = (
) => {
  return useMutation<PostApiMrReviewersData, Error, {link: string, data: ReviewerPayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.postApiMrReviewers().request(link, data)
  })
};