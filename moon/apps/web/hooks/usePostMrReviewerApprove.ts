import { useMutation } from "@tanstack/react-query";
import { ChangeReviewerStatePayload, PostApiMrReviewerApproveData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewerApprove = () => {
  return useMutation<PostApiMrReviewerApproveData, Error, {link: string, data: ChangeReviewerStatePayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.postApiMrReviewerApprove().request(link, data)
  })
}