import router from 'next/router'

import { GitMergeQueueIcon } from '@gitmono/ui/Icons'

import { useScope } from '@/contexts/scope'

import { SidebarLink } from './SidebarLink'

export function SidebarMergeQueue() {
  const { scope } = useScope()

  return (
    <>
      <SidebarLink
        id='mq'
        label='Merge Queue'
        href={`/${scope}/queue/main`}
        active={router.pathname === '/[org]/queue'}
        leadingAccessory={<GitMergeQueueIcon />}
      />
    </>
  )
}
