import { useState } from 'react'

import { InboxIcon } from '@gitmono/ui'

import { useRefetchInboxIndex } from '@/components/NavigationBar/useNavigationTabAction'
import { useScope } from '@/contexts/scope'
import { useGetNotifications } from '@/hooks/useGetNotifications'
import { useGetUnreadNotificationsCount } from '@/hooks/useGetUnreadNotificationsCount'

import { InboxHoverCard } from '../InboxItems/InboxHoverCard'
import { SidebarLink, SidebarProps } from './SidebarLink'
import { SidebarUnreadBadge } from './SidebarUnreadBadge'

export function SidebarInbox({ label = 'Inbox', href, active }: SidebarProps) {
  const { scope } = useScope()
  const getUnreadNotificationsCount = useGetUnreadNotificationsCount()
  const unreadInboxCount = getUnreadNotificationsCount.data?.home_inbox[`${scope}`] || 0
  const refetchInbox = useRefetchInboxIndex()

  const [prefetch, setPrefetch] = useState(false)

  useGetNotifications({ filter: 'home', enabled: prefetch })

  const unread = unreadInboxCount > 0

  return (
    <InboxHoverCard>
      <SidebarLink
        id='inbox'
        label={label}
        href={href}
        active={active}
        leadingAccessory={<InboxIcon />}
        unread={unread}
        trailingAccessory={
          unreadInboxCount > 0 && <SidebarUnreadBadge important={false}>{unreadInboxCount}</SidebarUnreadBadge>
        }
        onClick={() => {
          refetchInbox()
        }}
        onMouseEnter={() => setPrefetch(true)}
        onMouseLeave={() => setPrefetch(false)}
      />
    </InboxHoverCard>
  )
}
