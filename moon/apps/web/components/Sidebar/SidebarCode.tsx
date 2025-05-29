import router from 'next/router'

import { SidebarLink } from './SidebarLink'
import { useScope } from '@/contexts/scope'
import { ComponentIcon } from '@gitmono/ui/Icons'

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
