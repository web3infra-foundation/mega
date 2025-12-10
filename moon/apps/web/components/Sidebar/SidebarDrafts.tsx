import { useMemo } from 'react'

import { PostDraftIcon } from '@gitmono/ui/Icons'

import { SidebarLink, SidebarProps } from '@/components/Sidebar/SidebarLink'
import { SidebarUnreadBadge } from '@/components/Sidebar/SidebarUnreadBadge'
import { useGetPersonalDraftPosts } from '@/hooks/useGetPersonalDraftPosts'
import { flattenInfiniteData } from '@/utils/flattenInfiniteData'

export function SidebarDrafts({ label = 'Drafts', href, active }: SidebarProps) {
  const { data: draftPostsData } = useGetPersonalDraftPosts()
  const draftPosts = useMemo(() => flattenInfiniteData(draftPostsData) ?? [], [draftPostsData])

  if (draftPosts.length === 0 && !active) return null

  return (
    <SidebarLink
      id='drafts'
      label={label}
      href={href}
      active={active}
      leadingAccessory={<PostDraftIcon />}
      trailingAccessory={
        draftPosts.length > 0 ? (
          <SidebarUnreadBadge important={false}>{draftPosts.length}</SidebarUnreadBadge>
        ) : undefined
      }
    />
  )
}
