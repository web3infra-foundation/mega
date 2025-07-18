import { getMarkdownExtensions } from '@gitmono/editor'
import { RichTextRenderer } from '@/components/RichTextRenderer'
import { useMemo } from 'react'
import { Button, ConditionalWrap, FaceSmilePlusIcon, UIText } from '@gitmono/ui'
import { MemberHovercard } from '../InlinePost/MemberHovercard'
import { MemberAvatar } from '../MemberAvatar'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'
import { UserLinkByName } from './components/UserLinkByName'
import HandleTime from './components/HandleTime'
import { ReactionPicker } from '../Reactions/ReactionPicker'
import { useHandleExpression } from './hook/useHandleExpression'
import { CommentDropdownMenu } from './CommentDropdownMenu'
import { ConversationItem } from '@gitmono/types/generated'
import { ReactionShow } from '../Reactions/ReactionShow'

interface CommentProps {
  conv: ConversationItem
  id: string
  whoamI: string
}

const Comment = ({ conv, id, whoamI }: CommentProps) => {
  const { data: member } = useGetOrganizationMember({ username: conv.username })
  
  const extensions = useMemo(() => getMarkdownExtensions({ linkUnfurl: {} }), [])
  const handleReactionSelect = useHandleExpression({ conv, id, type: whoamI })

  return (
    <div className='overflow-hidden rounded-lg border border-gray-300 bg-white'>
      <div className='flex items-center justify-between border-b border-gray-300 py-2 px-4'>
        <div className='flex items-center space-x-3'>
          <div className='cursor-pointer'>
            <ConditionalWrap
              condition={true}
              wrap={(c) => (
                <MemberHovercard username={conv?.username}>
                  <UserLinkByName username={conv?.username} className='relative'>
                    {c}
                  </UserLinkByName>
                </MemberHovercard>
              )}
            >
              { member ? <MemberAvatar member={member} size='sm'/> : "Avatar not found"}
            </ConditionalWrap>
          </div>
          <div className='cursor-pointer'>
            <ConditionalWrap
              condition={true}
              wrap={(children) => (
                <MemberHovercard username={conv?.username}>
                  <UserLinkByName username={conv?.username}>{children}</UserLinkByName>
                </MemberHovercard>
              )}
            >
              <UIText element='span' primary weight='font-medium' className='break-anywhere line-clamp-1'>
                {conv?.username || 'username not found'}
              </UIText>
            </ConditionalWrap>
          </div>
          <div className='text-sm text-gray-500 hover:text-gray-700'>
            <HandleTime created_at={conv.created_at}/>
          </div>
        </div>
        <div className='flex items-center'>
          <ReactionPicker
            custom
            align='end'
            trigger={
              <Button
                variant='plain'
                iconOnly={<FaceSmilePlusIcon />}
                accessibilityLabel='Add reaction'
                tooltip='Add reaction'
              />
            }
            onReactionSelect={handleReactionSelect}
          />
          <CommentDropdownMenu id={id} Conversation={conv} CommentType={whoamI}/>
        </div>
      </div>

      <div className='prose copyable-text p-3'>
        { conv.comment && <RichTextRenderer content={conv.comment} extensions={extensions} /> }
        <ReactionShow comment={conv} id={id} type={whoamI} />
      </div>
    </div>
  )
}

export default Comment
