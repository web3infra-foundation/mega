import { useMutation } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";
import { DeleteApiMrReviewersData, ReviewerPayload } from "@gitmono/types";

export const useDeleteMrReviewers = () => {
  return useMutation<DeleteApiMrReviewersData, Error, {link: string, data: ReviewerPayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.deleteApiMrReviewers().request(link, data)
  })
}