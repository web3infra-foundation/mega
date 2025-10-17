import router from 'next/router'

import { SidebarLink } from './SidebarLink'
import { useScope } from '@/contexts/scope'
import { TagIcon } from '@gitmono/ui/Icons'

export function SidebarTags() {
  const { scope } = useScope()
  
  return (
   <>
     <SidebarLink
        id='tags'
        label='Tags'
        href={`/${scope}/code/tags`}
        active={router.pathname === '/[org]/code/tags'}
        leadingAccessory={<TagIcon />}
      />
   </>
  )
}

