import { getMarkdownExtensions } from '@gitmono/editor'
import { Conversation } from '@/pages/[org]/mr/[id]'
import { RichTextRenderer } from '@/components/RichTextRenderer'
import { useMemo } from 'react'
import { ConditionalWrap, UIText } from '@gitmono/ui'
import { MemberHovercard } from '../InlinePost/MemberHovercard'
import { MemberAvatar } from '../MemberAvatar'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'
import { KebabHorizontalIcon } from '@primer/octicons-react'
import { UserLinkByName } from './components/UserLinkByName'
import HandleTime from './components/HandleTime'

interface CommentProps {
  conv: Conversation
  id: string
  whoamI: string
}

const Comment = ({ conv }: CommentProps) => {
  const { data: member } = useGetOrganizationMember({ username: conv.username })
  
  const extensions = useMemo(() => getMarkdownExtensions({ linkUnfurl: {} }), [])

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
        <button className='hover:text-blue-700'>
          <KebabHorizontalIcon />
        </button>
      </div>

      <div className='prose copyable-text p-3'>
        <RichTextRenderer content={conv.comment} extensions={extensions} />
      </div>
    </div>
  )
}

export default Comment
