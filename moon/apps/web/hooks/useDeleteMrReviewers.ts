import { useMutation, useQueryClient } from "@tanstack/react-query";
import { legacyApiClient } from "@/utils/queryClient";
import { DeleteApiMrReviewersData, ReviewerPayload } from "@gitmono/types";

export const useDeleteMrReviewers = () => {
  const queryClient = useQueryClient()

  return useMutation<DeleteApiMrReviewersData, Error, {link: string, data: ReviewerPayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.deleteApiMrReviewers().request(link, data),
    onSuccess: (_, { link }) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrReviewers().requestKey(link)
      })
    }
  })
}
