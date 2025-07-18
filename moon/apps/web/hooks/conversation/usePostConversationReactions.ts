import { useMutation, useQueryClient } from '@tanstack/react-query'

import type { ReactionRequest } from '@gitmono/types'
import { SyncCustomReaction } from '@gitmono/types'

import { legacyApiClient } from '@/utils/queryClient'
import { getStandardReaction, StandardReaction } from '@/utils/reactions'

type Props = {
  reaction: StandardReaction | SyncCustomReaction
}

export function usePostConversationReactions(commentId: number, id: string, type: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ reaction }: Props) =>
      legacyApiClient.v1.postApiConversationReactions().request(commentId, {
        content: getStandardReaction(reaction)?.native,
        comment_type: type
      } as ReactionRequest),
    onSuccess: () => {
      switch (type) {
        case 'issue':
          queryClient.invalidateQueries({
            queryKey: legacyApiClient.v1.getApiIssueDetail().requestKey(id)
          })
          break
        case 'mr':
          queryClient.invalidateQueries({
            queryKey: legacyApiClient.v1.getApiMrDetail().requestKey(id)
          })
          break
        default:
          return
      }
    }
  })
}
