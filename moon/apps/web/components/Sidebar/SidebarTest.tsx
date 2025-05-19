import { useState } from 'react'
import router from 'next/router'

import { SidebarLink } from './SidebarLink'
import { useScope } from '@/contexts/scope'
import { ComponentIcon } from '@gitmono/ui/Icons'

export function SidebarTest() {
  const { scope } = useScope()
  
  return (
   <>
     <SidebarLink
        id='test'
        label='Test'
        href={`/${scope}/test`}
        active={router.pathname === '/[org]/test'}
        leadingAccessory={<ComponentIcon />}
      />
   </>
  )
}
