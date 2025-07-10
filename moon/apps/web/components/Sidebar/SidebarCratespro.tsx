import { ChatBubblePlusIcon } from '@gitmono/ui/Icons'
import { useRouter } from 'next/router'
import { useScope } from '@/contexts/scope'
import { SidebarLink } from './SidebarLink'

export function SidebarCratespro() {
  const { scope } = useScope()
  const router = useRouter()

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