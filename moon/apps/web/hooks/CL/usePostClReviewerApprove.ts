import { useMutation } from "@tanstack/react-query";
import { ChangeReviewerStatePayload, PostApiClReviewerApproveData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostClReviewerApprove = () => {
  return useMutation<PostApiClReviewerApproveData, Error, {link: string, data: ChangeReviewerStatePayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.postApiClReviewerApprove().request(link, data)
  })
}