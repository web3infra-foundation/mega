import { useMutation, useQueryClient } from '@tanstack/react-query'
import { legacyApiClient } from '@/utils/queryClient'
import type { DeleteApiMrCommentDeleteData, RequestParams } from '@gitmono/types'

export function useDeleteMrCommentDelete(id: string, params?: RequestParams) {
  const queryClient = useQueryClient()

  return useMutation<DeleteApiMrCommentDeleteData, Error, number>({
    mutationKey: legacyApiClient.v1.deleteApiMrCommentDelete().baseKey,
    mutationFn: (convId) =>
      legacyApiClient.v1.deleteApiMrCommentDelete().request(convId, params),
    onSuccess: (_data, _convId) => {
      queryClient.invalidateQueries({
        queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(id),
      })
    },
  })
}