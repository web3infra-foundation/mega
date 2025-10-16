import router from 'next/router'

import { SidebarLink } from './SidebarLink'
import { useScope } from '@/contexts/scope'
import { GitCommitIcon} from '@gitmono/ui/Icons'

export function SiderbarChangeList() {
  const { scope } = useScope()
  
  return (
   <>
     <SidebarLink
        id='cl'
        label='Change List'
        href={`/${scope}/cl`}
        active={router.pathname === '/[org]/cl'}
        leadingAccessory={<GitCommitIcon />}
      />
   </>
  )
}

