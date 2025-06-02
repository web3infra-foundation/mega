import { DeleteApiIssueCommentDeleteData, RequestParams } from '@gitmono/types/generated'
import { useMutation, useQueryClient } from '@tanstack/react-query'

import { legacyApiClient } from '@/utils/queryClient'

export function useDeleteIssueComment(id: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<DeleteApiIssueCommentDeleteData, Error, number>({
    mutationKey: legacyApiClient.v1.deleteApiIssueCommentDelete().baseKey,
    mutationFn: (convId) => legacyApiClient.v1.deleteApiIssueCommentDelete().request(convId, params),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiIssueDetail().requestKey(id)
      })
    }
  })
}
