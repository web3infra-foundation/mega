import { useMutation } from "@tanstack/react-query";
import { ChangeReviewStatePayload, PostApiMrReviewResolveData } from "@gitmono/types";
import { legacyApiClient } from "@/utils/queryClient";

export const usePostMrReviewResolve = () => {
  return useMutation<PostApiMrReviewResolveData, Error, {link: string, data: ChangeReviewStatePayload}>({
    mutationFn: ({link, data}) => legacyApiClient.v1.postApiMrReviewResolve().request(link, data)
  })
}