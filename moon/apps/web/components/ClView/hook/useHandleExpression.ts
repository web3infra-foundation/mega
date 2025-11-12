import { useCallback } from 'react'

import { SyncCustomReaction } from '@gitmono/types'
import { ConversationItem, GroupedReaction } from '@gitmono/types/generated'

import { useDeleteConversationReaction } from '@/hooks/conversation/useDeleteConversationReaction'
import { usePostConversationReactions } from '@/hooks/conversation/usePostConversationReactions'
import { findGroupedReaction, StandardReaction } from '@/utils/reactions'

export function useHandleExpression({ conv, id, type }: { conv: ConversationItem; id: string; type: string }) {
  const { mutate: postConversationReactions } = usePostConversationReactions(conv.id, id, type)
  const { mutate: deleteConversationReaction } = useDeleteConversationReaction(id, type)

  return useCallback(
    (reaction: StandardReaction | SyncCustomReaction) => {
      if (!conv) return

      const groupedReaction = findGroupedReaction(conv.grouped_reactions as unknown as GroupedReaction[], reaction)

      if (groupedReaction?.viewer_reaction_id) {
        deleteConversationReaction({ reactionId: groupedReaction.viewer_reaction_id })
      } else {
        postConversationReactions({ reaction })
      }
    },
    [conv, deleteConversationReaction, postConversationReactions]
  )
}
