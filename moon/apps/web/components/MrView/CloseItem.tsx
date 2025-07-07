import { ConditionalWrap } from '@gitmono/ui'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'
import { Conversation } from '@/pages/[org]/mr/[id]'
import { MemberHovercard } from '../InlinePost/MemberHovercard'
import { MemberAvatar } from '../MemberAvatar'
import { UserLinkByName } from './components/UserLinkByName'
import HandleTime from './components/HandleTime'


interface CloseItemProps {
  conv: Conversation
}
const CloseItem = ({ conv }: CloseItemProps) => {
  const { data: member } = useGetOrganizationMember({ username: conv.username })

  return (
    <>
      <div className='flex items-center space-x-2'>
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
            {member ? <MemberAvatar member={member} size='sm' /> : 'Avatar not found'}
          </ConditionalWrap>
        </div>
        <div>{conv.comment}</div>
        <div className='text-sm text-gray-500 hover:text-gray-700'>
            <HandleTime created_at={conv.created_at}/>
        </div>
      </div>
    </>
  )
}

export default CloseItem
