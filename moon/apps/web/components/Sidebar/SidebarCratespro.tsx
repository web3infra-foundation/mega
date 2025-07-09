import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'
import router from 'next/router'
import { useScope } from '@/contexts/scope'
import { SidebarLink } from './SidebarLink'

export function SidebarCratespro() {
  const { scope } = useScope()

  return (
    <SidebarLink
      id='cratespro'
      label='Cratespro'
      href={`/${scope}/cratespro`}
      active={router.pathname === '/[org]/cratespro'}
      leadingAccessory={<ChatBubblePlusIcon />}
    />
  )
}