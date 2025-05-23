import router from 'next/router'

import { SidebarLink } from './SidebarLink'
import { useScope } from '@/contexts/scope'
import { GitCommitIcon} from '@gitmono/ui/Icons'

export function SiderbarMergeRequest() {
  const { scope } = useScope()
  
  return (
   <>
     <SidebarLink
        id='mr'
        label='Merge Request'
        href={`/${scope}/mr`}
        active={router.pathname === '/[org]/mr'}
        leadingAccessory={<GitCommitIcon />}
      />
   </>
  )
}

