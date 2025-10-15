import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { DeleteApiConversationReactionsByIdData, RequestParams } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'

type Props = {
  reactionId: string
  params?: RequestParams
}

export function useDeleteConversationReaction(id: string, type: string) {
  const queryClient = useQueryClient()

  return useMutation<DeleteApiConversationReactionsByIdData, Error, Props>({
    mutationFn: async ({ reactionId, params }) =>
      legacyApiClient.v1.deleteApiConversationReactionsById().request(reactionId, { ...params }),
    onSuccess: () => {
      switch (type) {
        case 'issue':
          queryClient.invalidateQueries({
            queryKey: legacyApiClient.v1.getApiIssueDetail().requestKey(id)
          })
          break
        case 'cl':
          queryClient.invalidateQueries({
            queryKey: legacyApiClient.v1.getApiClDetail().requestKey(id)
          })
          break
        default:
          return
      }
    }
  })
}
