import router from 'next/router'

import { ComponentIcon } from '@gitmono/ui/Icons'

import { useScope } from '@/contexts/scope'

import { SidebarLink } from './SidebarLink'

export function SidebarCode() {
  const { scope } = useScope()

  return (
    <>
      <SidebarLink
        id='code'
        label='Code'
        href={`/${scope}/code`}
        active={router.pathname === '/[org]/code'}
        leadingAccessory={<ComponentIcon />}
      />
    </>
  )
}
