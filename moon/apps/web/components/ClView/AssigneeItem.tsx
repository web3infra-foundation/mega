import { ConditionalWrap } from '@gitmono/ui/utils'

import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'

import { MemberHovercard } from '../InlinePost/MemberHovercard'
import { MemberAvatar } from '../MemberAvatar'
import HandleTime from './components/HandleTime'
import { UserLinkByName } from './components/UserLinkByName'
import { ReopenItemProps } from './ReopenItem'

const AssigneeItem = ({ conv }: ReopenItemProps) => {
  const match = conv.comment?.match(/\["(.*?)"\]/) ?? ''
  const comment = conv.comment?.split(' ') ?? []
  const { data: member } = useGetOrganizationMember({ username: conv.username })

  const memberForAvatar =
    member?.user?.id && member.user.username && member.user.display_name && member.user.avatar_urls
      ? {
          deactivated: member.deactivated,
          user: {
            id: member.user.id,
            username: member.user.username,
            display_name: member.user.display_name,
            avatar_urls: member.user.avatar_urls,
            notifications_paused: member.user.notifications_paused ?? false,
            integration: member.user.integration ?? false
          }
        }
      : null

  const assignees = match[1].split('", "')

  return (
    <>
      <div className='flex flex-wrap items-center gap-2'>
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
          {memberForAvatar ? <MemberAvatar member={memberForAvatar} size='sm' /> : 'Avatar not found'}
        </ConditionalWrap>
        <div>
          <span className='font-semibold'>{conv.username} </span>
          <span className='text-gray-400'>{comment[1]} </span>
          {assignees &&
            assignees.map((i, index) => (
              <ConditionalWrap
                key={i}
                condition={true}
                wrap={(c) => (
                  <MemberHovercard username={i}>
                    <UserLinkByName username={i} className='relative'>
                      {c}
                    </UserLinkByName>
                  </MemberHovercard>
                )}
              >
                <>
                  <span className='cursor-pointer text-[#1f2328] underline'>
                    {i}
                    {index < assignees.length - 1 && ', '}
                  </span>
                </>
              </ConditionalWrap>
            ))}
        </div>
        <div className='text-sm text-gray-500 hover:text-gray-700'>
          <HandleTime created_at={conv.created_at} />
        </div>
      </div>
    </>
  )
}

export default AssigneeItem
