import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'
import router from 'next/router'

import { useScope } from '@/contexts/scope'

import { SidebarLink } from './SidebarLink'

export function SidebarIssue() {
  const { scope } = useScope()

  return (
    <>
      <SidebarLink
        id='Issue'
        label='Issue'
        href={`/${scope}/issue`}
        active={router.pathname === '/[org]/issue'}
        leadingAccessory={<ChatBubblePlusIcon />}
      />
    </>
  )
}
