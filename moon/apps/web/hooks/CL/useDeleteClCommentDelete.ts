import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { DeleteApiConversationByCommentIdData, RequestParams } from '@gitmono/types'

export function useDeleteClCommentDelete(id: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<DeleteApiConversationByCommentIdData, Error, number>({
    mutationKey: legacyApiClient.v1.deleteApiConversationByCommentId().baseKey,
    mutationFn: (convId) =>
      legacyApiClient.v1.deleteApiConversationByCommentId().request(convId, params),
    onSuccess: (_data, _convId) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiClDetail().requestKey(id),
      })
    },
  })
}