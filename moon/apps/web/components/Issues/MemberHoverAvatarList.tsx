import React from 'react'
import { Avatar, AvatarStack } from '@primer/react'
import { useQueries } from '@tanstack/react-query'

import { OrganizationMember } from '@gitmono/types/generated'

import { useScope } from '@/contexts/scope'
import { apiClient } from '@/utils/queryClient'

import { ItemsType } from './IssuesContent'
import { MemberHovercard } from './MemberHoverCardNE'

export const MemberHoverAvatarList = ({ users }: { users: ItemsType[number] }) => {
  const shouldFetch = users.assignees.length > 0
  const query = apiClient.organizations.getMembersByUsername()

  const { scope } = useScope()

  const queries = useQueries({
    queries: users.assignees.map((u) => ({
      queryKey: query.requestKey(`${scope}`, `${u}`),
      queryFn: () => query.request(`${scope}`, `${u}`),
      enabled: shouldFetch
    })),
    combine: (res) => {
      return {
        data: res.map((r) => r.data),
        pending: res.some((r) => r.isPending)
      }
    }
  })

  return (
    <>
      <AvatarStack alignRight>
        {queries.pending
          ? Array.from({ length: users.assignees.length }).map((_, i) => (
              // eslint-disable-next-line react/no-array-index-key
              <div className='h-[48px] w-[48px] rounded-full bg-[#f3f4f5]' key={i} />
            ))
          : queries.data.map(
              (q) =>
                q && (
                  <AvatarwithHover
                    key={q.id}
                    src={q?.user.avatar_url}
                    hoverProps={{ username: q.user.username, userData: q }}
                  />
                )
            )}
      </AvatarStack>
    </>
  )
}

interface HoverProps {
  username: string
  userData: OrganizationMember
}
const AvatarwithHover = ({
  src,
  hoverProps,
  className,
  style
}: {
  src: string
  hoverProps: HoverProps
  className?: string
  style?: React.CSSProperties
}) => {
  return (
    <>
      <MemberHovercard username={hoverProps.username} side='top' align='end' member={hoverProps.userData}>
        <div className={className} style={style}>
          <Avatar src={src} />
        </div>
      </MemberHovercard>
    </>
  )
}
