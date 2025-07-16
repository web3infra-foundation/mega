import { ConversationItem, GroupedReaction } from '@gitmono/types/generated'
import { Button, cn, FaceSmilePlusIcon } from '@gitmono/ui'

import { Reactions } from '.'
import { useHandleExpression } from '../MrView/hook/useHandleExpression'
import { ReactionPicker } from './ReactionPicker'

export function ReactionShow({ comment, id, type }: { comment: ConversationItem, id: string, type: string }) {
  const handleReactionSelect = useHandleExpression({ conv: comment, id: id, type: type })

  if (!comment.grouped_reactions.length) return null

  function getClasses(hasReacted: boolean) {
    return cn(
      'flex gap-[5px] pointer-events-auto items-center p-0.5 pl-1.5 pr-2 justify-center group h-6.5 rounded-full text-xs font-semibold ring-1 min-w-[32px]',
      {
        'bg-blue-50 dark:bg-blue-900/40 hover:bg-blue-100 dark:hover:bg-blue-900/60 text-blue-900 dark:text-blue-400':
          hasReacted,
        'bg-tertiary hover:bg-quaternary': !hasReacted
      }
    )
  }

  return (
    <div className='flex flex-col items-start gap-2 pt-1'>
      <div className='-ml-1 flex flex-wrap items-center gap-1'>
        <ReactionPicker
          custom
          trigger={
            <Button
              className='hover:text-primary text-tertiary px-0.5'
              round
              size='sm'
              variant='plain'
              iconOnly={<FaceSmilePlusIcon />}
              accessibilityLabel='Add reaction'
            />
          }
          onReactionSelect={handleReactionSelect}
        />
        <Reactions
          reactions={comment.grouped_reactions as unknown as GroupedReaction[]}
          onReactionSelect={handleReactionSelect}
          getClasses={getClasses}
        />
      </div>
    </div>
  )
}
